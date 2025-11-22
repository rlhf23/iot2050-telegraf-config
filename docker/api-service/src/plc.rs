use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

// rust7 S7 client
use rust7::client::S7Client;

/// PLC connection configuration
#[derive(Clone)]
pub struct PlcConfig {
    pub ip: String,
    pub rack: u16,
    pub slot: u16,
    pub db_number: u16,
}

impl Default for PlcConfig {
    fn default() -> Self {
        Self {
            ip: "192.168.0.1".to_string(),
            rack: 0,
            slot: 1,
            db_number: 100,
        }
    }
}

/// Shared PLC service state
pub struct PlcService {
    config: PlcConfig,
    client: Arc<Mutex<Option<S7Client>>>,
}

impl PlcService {
    pub fn new(config: PlcConfig) -> Self {
        Self {
            config,
            client: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to PLC (lazy initialization)
    async fn ensure_connected(&self) -> Result<(), String> {
        let mut client_guard = self.client.lock().await;
        
        if client_guard.is_none() {
            info!("Connecting to PLC at {}...", self.config.ip);
            
            let mut client = S7Client::new();
            
            // Connect to PLC (S7-1200/1500 protocol)
            match client.connect_s71200_1500(&self.config.ip) {
                Ok(_) => {
                    info!("✓ Connected to PLC");
                    *client_guard = Some(client);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to connect to PLC: {}", e);
                    Err(format!("PLC connection failed: {}", e))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Write a single bit to the command word
    pub async fn write_command_bit(&self, bit_index: u8, value: bool) -> Result<(), String> {
        if bit_index >= 16 {
            return Err("Bit index must be 0-15".to_string());
        }

        self.ensure_connected().await?;
        
        let mut client_guard = self.client.lock().await;
        let client = client_guard.as_mut().ok_or("Client not connected")?;

        // Read current 2-byte command word
        let mut buffer = vec![0u8; 2];
        client
            .read_db(self.config.db_number, 0, &mut buffer)
            .map_err(|e| format!("Failed to read DB: {}", e))?;

        // Modify the bit
        let byte_index = (bit_index / 8) as usize;
        let bit_offset = bit_index % 8;
        
        if value {
            buffer[byte_index] |= 1 << bit_offset;
        } else {
            buffer[byte_index] &= !(1 << bit_offset);
        }

        // Write back
        client
            .write_db(self.config.db_number, 0, &buffer)
            .map_err(|e| format!("Failed to write DB: {}", e))?;

        info!("Wrote bit {} = {} to DB{}", bit_index, value, self.config.db_number);
        Ok(())
    }

    /// Pulse a command bit (set high, wait, set low)
    pub async fn pulse_command_bit(&self, bit_index: u8, duration_ms: u64) -> Result<(), String> {
        self.write_command_bit(bit_index, true).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        self.write_command_bit(bit_index, false).await?;
        Ok(())
    }

    /// Read status bits from offset 2 (bytes 2-3)
    pub async fn read_status_bits(&self) -> Result<Vec<bool>, String> {
        self.ensure_connected().await?;
        
        let mut client_guard = self.client.lock().await;
        let client = client_guard.as_mut().ok_or("Client not connected")?;

        // Read 2 bytes starting at offset 2
        let mut buffer = vec![0u8; 2];
        client
            .read_db(self.config.db_number, 2, &mut buffer)
            .map_err(|e| format!("Failed to read status: {}", e))?;

        // Convert to bool array
        let mut bits = Vec::with_capacity(16);
        for byte in &buffer {
            for bit_offset in 0..8 {
                bits.push((byte & (1 << bit_offset)) != 0);
            }
        }

        Ok(bits)
    }
}

// === API Request/Response Types ===

#[derive(Deserialize)]
pub struct CommandRequest {
    pub bit: u8,
    #[serde(default)]
    pub pulse: bool,
    #[serde(default = "default_pulse_duration")]
    pub pulse_duration_ms: u64,
}

fn default_pulse_duration() -> u64 {
    100
}

#[derive(Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub success: bool,
    pub bits: Vec<bool>,
}

// === Axum Route Handlers ===

pub async fn handle_command(
    State(state): State<crate::AppState>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, (StatusCode, Json<CommandResponse>)> {
    let plc = match &state.plc {
        Some(plc) => plc,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(CommandResponse {
                    success: false,
                    message: "PLC service not configured".to_string(),
                }),
            ));
        }
    };

    info!("PLC command: bit={}, pulse={}", req.bit, req.pulse);

    let result = if req.pulse {
        plc.pulse_command_bit(req.bit, req.pulse_duration_ms).await
    } else {
        plc.write_command_bit(req.bit, true).await
    };

    match result {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Command bit {} executed", req.bit),
        })),
        Err(e) => {
            error!("PLC command failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CommandResponse {
                    success: false,
                    message: e,
                }),
            ))
        }
    }
}

pub async fn handle_status(
    State(state): State<crate::AppState>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<StatusResponse>)> {
    let plc = match &state.plc {
        Some(plc) => plc,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(StatusResponse {
                    success: false,
                    bits: vec![],
                }),
            ));
        }
    };

    match plc.read_status_bits().await {
        Ok(bits) => Ok(Json(StatusResponse {
            success: true,
            bits,
        })),
        Err(e) => {
            error!("Failed to read PLC status: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StatusResponse {
                    success: false,
                    bits: vec![],
                }),
            ))
        }
    }
}
