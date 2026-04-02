use serde::Deserialize;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Invalid address format: {0}")]
    InvalidAddress(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlConfig {
    pub plc: PlcConfig,
    pub buttons: Vec<ButtonDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlcConfig {
    pub ip: String,
    pub rack: u16,
    pub slot: u16,
    #[serde(default = "default_timeout")]
    pub connection_timeout_ms: u64,
    #[serde(default = "default_read_timeout")]
    pub read_timeout_ms: u64,
    #[serde(default = "default_write_timeout")]
    pub write_timeout_ms: u64,
}

fn default_timeout() -> u64 {
    2000
}
fn default_read_timeout() -> u64 {
    2000
}
fn default_write_timeout() -> u64 {
    2000
}

#[derive(Debug, Clone, Deserialize)]
pub struct ButtonDef {
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub mode: ButtonMode,
    #[serde(default = "default_momentary_duration")]
    pub momentary_duration_ms: u64,
}

fn default_momentary_duration() -> u64 {
    500
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonMode {
    #[serde(rename = "toggle")]
    Toggle,
    #[serde(rename = "momentary")]
    Momentary,
}

impl Default for ButtonMode {
    fn default() -> Self {
        ButtonMode::Toggle
    }
}

#[derive(Debug, Clone)]
pub struct ParsedAddress {
    pub db: i32,
    pub byte: i32,
    pub bit: i32,
}

impl ControlConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: ControlConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn get_plc_ip(&self) -> Result<std::net::Ipv4Addr, ConfigError> {
        self.plc
            .ip
            .parse()
            .map_err(|_| ConfigError::InvalidAddress(format!("Invalid IP: {}", self.plc.ip)))
    }
}

impl ButtonDef {
    pub fn parse_address(&self) -> Result<ParsedAddress, ConfigError> {
        parse_s7_address(&self.address)
    }
}

pub fn parse_s7_address(addr: &str) -> Result<ParsedAddress, ConfigError> {
    let addr = addr.to_uppercase();
    if !addr.starts_with("DB") {
        return Err(ConfigError::InvalidAddress(format!(
            "Address must start with 'DB': {}",
            addr
        )));
    }

    let rest = &addr[2..];
    let dot_pos = rest.find('.').ok_or_else(|| {
        ConfigError::InvalidAddress(format!("Missing separator in address: {}", addr))
    })?;

    let db: i32 = rest[..dot_pos]
        .parse()
        .map_err(|_| ConfigError::InvalidAddress(format!("Invalid DB number: {}", addr)))?;

    let after_dot = &rest[dot_pos + 1..];

    if after_dot.starts_with("DBX") {
        let bit_addr = &after_dot[3..];
        let parts: Vec<&str> = bit_addr.split('.').collect();
        if parts.len() != 2 {
            return Err(ConfigError::InvalidAddress(format!(
                "Invalid bit address format: {}",
                addr
            )));
        }
        let byte: i32 = parts[0]
            .parse()
            .map_err(|_| ConfigError::InvalidAddress(format!("Invalid byte: {}", addr)))?;
        let bit: i32 = parts[1]
            .parse()
            .map_err(|_| ConfigError::InvalidAddress(format!("Invalid bit: {}", addr)))?;

        if bit < 0 || bit > 7 {
            return Err(ConfigError::InvalidAddress(format!(
                "Bit must be 0-7: {}",
                addr
            )));
        }

        Ok(ParsedAddress { db, byte, bit })
    } else if after_dot.starts_with("DBB") {
        let byte: i32 = after_dot[3..]
            .parse()
            .map_err(|_| ConfigError::InvalidAddress(format!("Invalid byte: {}", addr)))?;
        Ok(ParsedAddress { db, byte, bit: 0 })
    } else {
        // Try format: DB1.0.0 (db.byte.bit)
        let parts: Vec<&str> = after_dot.split('.').collect();
        if parts.len() == 2 {
            let byte: i32 = parts[0]
                .parse()
                .map_err(|_| ConfigError::InvalidAddress(format!("Invalid byte: {}", addr)))?;
            let bit: i32 = parts[1]
                .parse()
                .map_err(|_| ConfigError::InvalidAddress(format!("Invalid bit: {}", addr)))?;
            if bit < 0 || bit > 7 {
                return Err(ConfigError::InvalidAddress(format!(
                    "Bit must be 0-7: {}",
                    addr
                )));
            }
            Ok(ParsedAddress { db, byte, bit })
        } else if parts.len() == 1 {
            let byte: i32 = parts[0]
                .parse()
                .map_err(|_| ConfigError::InvalidAddress(format!("Invalid byte: {}", addr)))?;
            Ok(ParsedAddress { db, byte, bit: 0 })
        } else {
            Err(ConfigError::InvalidAddress(format!(
                "Invalid address format: {}",
                addr
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dbx_address() {
        let addr = parse_s7_address("DB1.DBX0.0").unwrap();
        assert_eq!(addr.db, 1);
        assert_eq!(addr.byte, 0);
        assert_eq!(addr.bit, 0);

        let addr = parse_s7_address("DB10.DBX2.3").unwrap();
        assert_eq!(addr.db, 10);
        assert_eq!(addr.byte, 2);
        assert_eq!(addr.bit, 3);
    }

    #[test]
    fn test_parse_short_address() {
        let addr = parse_s7_address("DB1.0.0").unwrap();
        assert_eq!(addr.db, 1);
        assert_eq!(addr.byte, 0);
        assert_eq!(addr.bit, 0);

        let addr = parse_s7_address("DB5.10.7").unwrap();
        assert_eq!(addr.db, 5);
        assert_eq!(addr.byte, 10);
        assert_eq!(addr.bit, 7);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let addr = parse_s7_address("db1.dbx0.0").unwrap();
        assert_eq!(addr.db, 1);
        assert_eq!(addr.byte, 0);
        assert_eq!(addr.bit, 0);
    }

    #[test]
    fn test_invalid_address() {
        assert!(parse_s7_address("M0.0").is_err());
        assert!(parse_s7_address("DB1").is_err());
        assert!(parse_s7_address("DB1.DBX0.8").is_err());
    }
}
