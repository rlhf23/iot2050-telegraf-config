use crate::backend::format::{
    format_config, format_config_header, parse_xml, NamespaceInfo, OpcuaConfig, OutputFormat,
};
use crate::error::TelegrafError;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_config_header_influxdb() {
        let influx_token = "test_token";
        let bucket_name = "test_bucket";
        let config_strings = vec!["config1".to_string(), "config2".to_string()];
        let namespace_infos = vec![
            NamespaceInfo {
                number: "1".to_string(),
                file_name: "test1.xml".to_string(),
            },
            NamespaceInfo {
                number: "2".to_string(),
                file_name: "test2.xml".to_string(),
            },
        ];
        let output_format = OutputFormat::InfluxDB;
        let include_test_inputs = false;

        let result = format_config_header(
            influx_token,
            bucket_name,
            &config_strings,
            &namespace_infos,
            output_format,
            include_test_inputs,
        );

        assert!(result.contains("# Namespace for file test1.xml: 1"));
        assert!(result.contains("# Namespace for file test2.xml: 2"));
        assert!(result.contains("token = \"test_token\""));
        assert!(result.contains("bucket = \"test_bucket\""));
        assert!(result.contains("config1"));
        assert!(result.contains("config2"));
        assert!(result.contains("[[outputs.influxdb_v2]]"));
        assert!(!result.contains("[[outputs.prometheus_client]]"));
    }

    #[test]
    fn test_format_config_header_prometheus() {
        let influx_token = ""; // Not needed for Prometheus
        let bucket_name = ""; // Not needed for Prometheus
        let config_strings = vec!["config1".to_string()];
        let namespace_infos = vec![NamespaceInfo {
            number: "1".to_string(),
            file_name: "test1.xml".to_string(),
        }];
        let output_format = OutputFormat::Prometheus;
        let include_test_inputs = false;

        let result = format_config_header(
            influx_token,
            bucket_name,
            &config_strings,
            &namespace_infos,
            output_format,
            include_test_inputs,
        );

        assert!(result.contains("# Namespace for file test1.xml: 1"));
        assert!(result.contains("[[outputs.prometheus_client]]"));
        assert!(result.contains("listen = \":9273\""));
        assert!(!result.contains("[[outputs.influxdb_v2]]"));
    }

    #[test]
    fn test_format_config_header_with_test_inputs() {
        let config_strings = vec!["config1".to_string()];
        let namespace_infos = vec![NamespaceInfo {
            number: "1".to_string(),
            file_name: "test1.xml".to_string(),
        }];
        let output_format = OutputFormat::InfluxDB;
        let include_test_inputs = true;

        let result = format_config_header(
            "token",
            "bucket",
            &config_strings,
            &namespace_infos,
            output_format,
            include_test_inputs,
        );

        assert!(result.contains("[[inputs.cpu]]"));
        assert!(result.contains("[[inputs.disk]]"));
        assert!(result.contains("[[inputs.mem]]"));
        assert!(result.contains("TEST INPUTS START"));
        assert!(result.contains("TEST INPUTS END"));
    }

    #[test]
    fn test_parse_xml_with_simple_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_simple.xml");

        // Create a simple XML file for testing
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <UANodeSet>
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2">
                <BrowseName>Temperature</BrowseName>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=3">
                <BrowseName>Pressure</BrowseName>
            </UAVariable>
        </UANodeSet>"#;

        let mut file = File::create(&file_path)?;
        file.write_all(xml_content.as_bytes())?;

        let config = OpcuaConfig {
            ip: "192.168.1.100:4840",
            username: "user",
            password: "pass",
            is_listener: false,
            group_name: "",
            namespace_number: "2",
            identifier_type: "i",
            interval_ms: 1000,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_ok());
        let config_str = result.unwrap();

        // Check that the parsed config contains expected elements
        assert!(config_str.contains("endpoint = \"opc.tcp://192.168.1.100:4840\""));
        assert!(config_str.contains("username = \"user\""));
        assert!(config_str.contains("password = \"pass\""));
        assert!(config_str.contains("name = \"TestDevice\""));
        assert!(config_str.contains("namespace = \"2\""));
        assert!(config_str.contains("interval = \"1000ms\""));
        assert!(config_str.contains("name=\"Temperature\", identifier=\"2\""));
        assert!(config_str.contains("name=\"Pressure\", identifier=\"3\""));

        assert_eq!(namespace_infos.len(), 1);
        assert_eq!(namespace_infos[0].number, "2");

        Ok(())
    }

    #[test]
    fn test_parse_xml_with_duplicate_nodes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_duplicates.xml");

        // Create an XML file with duplicate node names for testing
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <UANodeSet>
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2">
                <BrowseName>DuplicateName</BrowseName>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=3">
                <BrowseName>DuplicateName</BrowseName>
            </UAVariable>
        </UANodeSet>"#;

        let mut file = File::create(&file_path)?;
        file.write_all(xml_content.as_bytes())?;

        let config = OpcuaConfig {
            ip: "192.168.1.100:4840",
            username: "user",
            password: "pass",
            is_listener: false,
            group_name: "",
            namespace_number: "2",
            identifier_type: "i",
            interval_ms: 1000,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_err());
        if let Err(TelegrafError::DuplicateNodeError(msg)) = result {
            assert!(msg.contains("Duplicate node name"));
            assert!(msg.contains("DuplicateName"));
        } else {
            panic!("Expected DuplicateNodeError but got a different error or success");
        }

        Ok(())
    }

    #[test]
    fn test_parse_xml_with_variable_mapping() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_variable_mapping.xml");

        // Create an XML file with VariableMapping for testing
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <UANodeSet>
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2">
                <BrowseName>Temperature</BrowseName>
                <VariableMapping>"MappedTemperature"</VariableMapping>
            </UAVariable>
        </UANodeSet>"#;

        let mut file = File::create(&file_path)?;
        file.write_all(xml_content.as_bytes())?;

        let config = OpcuaConfig {
            ip: "192.168.1.100:4840",
            username: "user",
            password: "pass",
            is_listener: false,
            group_name: "",
            namespace_number: "2",
            identifier_type: "i",
            interval_ms: 1000,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_ok());
        let config_str = result.unwrap();

        // Check that the variable mapping is used instead of BrowseName
        assert!(config_str.contains("name=\"MappedTemperature\", identifier=\"2\""));
        assert!(!config_str.contains("name=\"Temperature\", identifier=\"2\""));

        Ok(())
    }
    
    #[test]
    fn test_format_config_regular() {
        let config = OpcuaConfig {
            ip: "192.168.1.100:4840",
            username: "user",
            password: "pass",
            is_listener: false,
            group_name: "test_group",
            namespace_number: "2",
            identifier_type: "i",
            interval_ms: 1000,
        };
        
        let nodes_str = "test_node";
        let result = format_config(&config, nodes_str);
        
        assert!(result.contains("endpoint = \"opc.tcp://192.168.1.100:4840\""));
        assert!(result.contains("username = \"user\""));
        assert!(result.contains("password = \"pass\""));
        assert!(result.contains("name = \"test_group\""));
        assert!(result.contains("namespace = \"2\""));
        assert!(result.contains("interval = \"1000ms\""));
        assert!(result.contains("test_node"));
    }
    
    #[test]
    fn test_format_config_listener() {
        let config = OpcuaConfig {
            ip: "192.168.1.100:4840",
            username: "user",
            password: "pass",
            is_listener: true,
            group_name: "test_group",
            namespace_number: "2",
            identifier_type: "i",
            interval_ms: 1000,
        };
        
        let nodes_str = "test_node";
        let result = format_config(&config, nodes_str);
        
        assert!(result.contains("endpoint = \"opc.tcp://192.168.1.100:4840\""));
        assert!(result.contains("username = \"user\""));
        assert!(result.contains("password = \"pass\""));
        assert!(result.contains("name = \"test_group\""));
        assert!(result.contains("namespace = \"2\""));
        assert!(result.contains("interval = \"1000ms\""));
        assert!(result.contains("test_node"));
        assert!(result.contains("[[inputs.opcua_listener]]"));
    }
}
