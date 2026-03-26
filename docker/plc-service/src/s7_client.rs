use crate::buttons::{ParsedAddress, PlcConfig};
use s7::{
    client::Client,
    field::{Bool, Field},
    tcp,
    transport::Connection,
};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum PlcError {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] std::io::Error),
    #[error("S7 protocol error: {0}")]
    S7Error(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Not connected")]
    NotConnected,
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, PlcError>;

pub struct S7Client {
    config: PlcConfig,
    transport: Option<Client<tcp::Transport>>,
}

impl S7Client {
    pub fn new(config: PlcConfig) -> Self {
        Self {
            config,
            transport: None,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        let ip: Ipv4Addr = self
            .config
            .ip
            .parse()
            .map_err(|_| PlcError::ConfigError(format!("Invalid IP: {}", self.config.ip)))?;

        let mut opts = tcp::Options::new(
            IpAddr::from(ip),
            self.config.rack,
            self.config.slot,
            Connection::PG,
        );

        opts.read_timeout = Duration::from_millis(self.config.read_timeout_ms);
        opts.write_timeout = Duration::from_millis(self.config.write_timeout_ms);

        let transport = tcp::Transport::connect(opts).map_err(|e| {
            PlcError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                e.to_string(),
            ))
        })?;

        let client = Client::new(transport).map_err(|e| PlcError::S7Error(e.to_string()))?;

        self.transport = Some(client);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.transport = None;
    }

    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
    }

    fn ensure_connected(&mut self) -> Result<&mut Client<tcp::Transport>> {
        if self.transport.is_none() {
            self.connect()?;
        }
        self.transport
            .as_mut()
            .ok_or(PlcError::NotConnected)
    }

    pub fn read_bool(&mut self, addr: &ParsedAddress) -> Result<bool> {
        let client = self.ensure_connected()?;

        let mut buffer = vec![0u8; 1];

        client
            .ag_read(addr.db, addr.byte, 1, &mut buffer)
            .map_err(|e| PlcError::S7Error(e.to_string()))?;

        let field = Bool::new(addr.db, addr.byte as f32 + (addr.bit as f32 / 10.0), buffer)
            .map_err(|e| PlcError::ParseError(e.to_string()))?;

        Ok(field.value())
    }

    pub fn write_bool(&mut self, addr: &ParsedAddress, value: bool) -> Result<()> {
        let client = self.ensure_connected()?;

        let mut buffer = vec![0u8; 1];

        client
            .ag_read(addr.db, addr.byte, 1, &mut buffer)
            .map_err(|e| PlcError::S7Error(e.to_string()))?;

        let mut field = Bool::new(addr.db, addr.byte as f32 + (addr.bit as f32 / 10.0), buffer)
            .map_err(|e| PlcError::ParseError(e.to_string()))?;

        field.set_value(value);
        let bytes = field.to_bytes();

        client
            .ag_write(addr.db, addr.byte, 1, &mut bytes.to_vec())
            .map_err(|e| PlcError::S7Error(e.to_string()))?;

        Ok(())
    }

    pub fn toggle_bool(&mut self, addr: &ParsedAddress) -> Result<bool> {
        let current = self.read_bool(addr)?;
        let new_value = !current;
        self.write_bool(addr, new_value)?;
        Ok(new_value)
    }

    pub fn get_cpu_status(&mut self) -> Result<String> {
        let client = self.ensure_connected()?;
        let _status = client
            .plc_status()
            .map_err(|e| PlcError::S7Error(e.to_string()))?;
        // If we got here, the PLC responded - return connected status
        // Note: CpuStatus enum is not publicly exported by the s7 crate
        Ok("Connected".to_string())
    }

    /// Check if connection to PLC is actually alive by trying to get CPU status
    pub fn check_connection(&mut self) -> bool {
        // If we don't have a connection object, try to connect
        if self.transport.is_none() {
            if self.connect().is_err() {
                return false;
            }
        }

        // Try to communicate with PLC
        match self.get_cpu_status() {
            Ok(_) => true,
            Err(_) => {
                // Connection failed, clear the transport
                self.transport = None;
                false
            }
        }
    }
}

#[derive(Clone)]
pub struct PlcClientManager {
    inner: Arc<RwLock<S7Client>>,
    config: Arc<PlcConfig>,
}

impl PlcClientManager {
    pub fn new(config: PlcConfig) -> Self {
        let client = S7Client::new(config.clone());
        Self {
            inner: Arc::new(RwLock::new(client)),
            config: Arc::new(config),
        }
    }

    pub async fn read_bool(&self, addr: &ParsedAddress) -> Result<bool> {
        let mut client = self.inner.write().await;
        client.read_bool(addr)
    }

    pub async fn write_bool(&self, addr: &ParsedAddress, value: bool) -> Result<()> {
        let mut client = self.inner.write().await;
        client.write_bool(addr, value)
    }

    pub async fn toggle_bool(&self, addr: &ParsedAddress) -> Result<bool> {
        let mut client = self.inner.write().await;
        client.toggle_bool(addr)
    }

    pub async fn is_connected(&self) -> bool {
        let client = self.inner.read().await;
        client.is_connected()
    }

    pub async fn reconnect(&self) -> Result<()> {
        let mut client = self.inner.write().await;
        client.disconnect();
        client.connect()
    }

    /// Check actual PLC connection by trying to communicate
    pub async fn check_connection(&self) -> bool {
        let mut client = self.inner.write().await;
        client.check_connection()
    }

    pub async fn status(&self) -> PlcStatus {
        // Actually verify connection by trying to communicate with PLC
        let connected = self.check_connection().await;
        PlcStatus {
            connected,
            ip: self.config.ip.clone(),
            rack: self.config.rack,
            slot: self.config.slot,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PlcStatus {
    pub connected: bool,
    pub ip: String,
    pub rack: u16,
    pub slot: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_address() {
        let addr = ParsedAddress {
            db: 1,
            byte: 0,
            bit: 0,
        };
        assert_eq!(addr.db, 1);
        assert_eq!(addr.byte, 0);
        assert_eq!(addr.bit, 0);
    }
}