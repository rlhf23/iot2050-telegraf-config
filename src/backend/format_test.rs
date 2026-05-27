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
        let include_diagnostics = false;

        let result = format_config_header(
            &config_strings,
            &namespace_infos,
            output_format,
            include_test_inputs,
            include_diagnostics,
            None,
        );

        assert!(result.contains("# Namespace for file test1.xml: 1"));
        assert!(result.contains("# Namespace for file test2.xml: 2"));
        assert!(result.contains("token = \"${INFLUXDB_TOKEN}\""));
        assert!(result.contains("bucket = \"${INFLUXDB_BUCKET}\""));
        assert!(result.contains("config1"));
        assert!(result.contains("config2"));
        assert!(result.contains("[[outputs.influxdb_v2]]"));
    }

    #[test]
    fn test_format_config_header_prometheus() {
        let config_strings = vec!["config1".to_string()];
        let namespace_infos = vec![NamespaceInfo {
            number: "1".to_string(),
            file_name: "test1.xml".to_string(),
        }];
        let output_format = OutputFormat::Prometheus;
        let include_test_inputs = false;
        let include_diagnostics = false;

        let result = format_config_header(
            &config_strings,
            &namespace_infos,
            output_format,
            include_test_inputs,
            include_diagnostics,
            None,
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
        let include_diagnostics = false;

        let result = format_config_header(
            &config_strings,
            &namespace_infos,
            output_format,
            include_test_inputs,
            include_diagnostics,
            None,
        );

        assert!(result.contains("[[inputs.cpu]]"));
        assert!(result.contains("[[inputs.disk]]"));
        assert!(result.contains("[[inputs.mem]]"));
        assert!(result.contains("TEST INPUTS START"));
        assert!(result.contains("TEST INPUTS END"));
        assert!(result.contains("INFLUXDB_DIAGNOSTICS_BUCKET"));
    }

    #[test]
    fn test_parse_xml_with_simple_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_simple.xml");

        // Create a simple XML file for testing
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <UANodeSet xmlns:si="http://www.siemens.com/OPCUA/2017/SimaticNodeSetExtensions">
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2" BrowseName="Temperature">
                <DisplayName>Temperature</DisplayName>
                <Extensions>
                    <Extension>
                        <si:VariableMapping>"TestDevice"."Temperature"</si:VariableMapping>
                    </Extension>
                </Extensions>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=3" BrowseName="Pressure">
                <DisplayName>Pressure</DisplayName>
                <Extensions>
                    <Extension>
                        <si:VariableMapping>"TestDevice"."Pressure"</si:VariableMapping>
                    </Extension>
                </Extensions>
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
            use_source_timestamp: false,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_ok());
        let parse_result = result.unwrap();

        // Check that the parsed config contains expected elements
        assert!(parse_result
            .config_string
            .contains("endpoint = \"opc.tcp://192.168.1.100:4840\""));
        assert!(parse_result.config_string.contains("username = \"user\""));
        assert!(parse_result.config_string.contains("password = \"pass\""));
        assert!(parse_result.config_string.contains("name = \"TestDevice\""));
        assert!(parse_result.config_string.contains("namespace = \"2\""));
        assert!(parse_result.config_string.contains("interval = \"1000ms\""));
        assert!(parse_result
            .config_string
            .contains("name=\"TestDevice.Temperature\", identifier=\"2\""));
        assert!(parse_result
            .config_string
            .contains("name=\"TestDevice.Pressure\", identifier=\"3\""));

        // Check measurement name was extracted
        assert_eq!(parse_result.measurement_name, "TestDevice");

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
        <UANodeSet xmlns:si="http://www.siemens.com/OPCUA/2017/SimaticNodeSetExtensions">
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2" BrowseName="DuplicateName">
                <DisplayName>DuplicateName</DisplayName>
                <Extensions>
                    <Extension>
                        <si:VariableMapping>"TestDevice"."DuplicateVar"</si:VariableMapping>
                    </Extension>
                </Extensions>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=3" BrowseName="DuplicateName2">
                <DisplayName>DuplicateName2</DisplayName>
                <Extensions>
                    <Extension>
                        <si:VariableMapping>"TestDevice"."DuplicateVar"</si:VariableMapping>
                    </Extension>
                </Extensions>
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
            use_source_timestamp: false,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_err());
        if let Err(TelegrafError::DuplicateNodeError(msg)) = result {
            assert!(msg.contains("Duplicate node name"));
            assert!(msg.contains("TestDevice.DuplicateVar"));
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
            use_source_timestamp: false,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_ok());
        let parse_result = result.unwrap();

        // Check that the variable mapping is used instead of BrowseName
        assert!(parse_result
            .config_string
            .contains("name=\"MappedTemperature\", identifier=\"2\""));
        assert!(!parse_result
            .config_string
            .contains("name=\"Temperature\", identifier=\"2\""));

        // Check measurement name
        assert_eq!(parse_result.measurement_name, "TestDevice");

        Ok(())
    }

    #[test]
    fn test_parse_xml_with_browse_name_attribute() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_browse_name_attr.xml");

        let xml_content = r#"<?xml version="1.0" encoding="utf-8"?>
        <UANodeSet xmlns:si="http://www.siemens.com/OPCUA/2017/SimaticNodeSetExtensions">
            <UAObject NodeId="ns=2;i=1" BrowseName="2:Motor">
                <DisplayName>Motor</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=12" BrowseName="2:Speed" DataType="REAL" AccessLevel="3">
                <DisplayName>Speed</DisplayName>
                <References>
                    <Reference ReferenceType="HasTypeDefinition" IsForward="true">i=63</Reference>
                </References>
                <Extensions>
                    <Extension>
                        <si:VariableMapping>"Motor"."Drive"."Speed"</si:VariableMapping>
                    </Extension>
                </Extensions>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=23" BrowseName="2:Torque" DataType="REAL" AccessLevel="3">
                <DisplayName>Torque</DisplayName>
                <References>
                    <Reference ReferenceType="HasTypeDefinition" IsForward="true">i=63</Reference>
                </References>
                <Extensions>
                    <Extension>
                        <si:VariableMapping>"Motor"."Drive"."Torque"</si:VariableMapping>
                    </Extension>
                </Extensions>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=99" BrowseName="NamespaceUri" ParentNodeId="ns=2;i=211" DataType="i=12" AccessLevel="1">
                <DisplayName>NamespaceUri</DisplayName>
                <References>
                    <Reference ReferenceType="HasTypeDefinition" IsForward="true">i=68</Reference>
                </References>
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
            use_source_timestamp: false,
        };

        let mut namespace_infos = Vec::new();
        let result = parse_xml(&config, file_path.to_str().unwrap(), &mut namespace_infos);

        assert!(result.is_ok());
        let parse_result = result.unwrap();
        assert!(parse_result
            .config_string
            .contains("name=\"Motor.Drive.Speed\", identifier=\"12\""));
        assert!(parse_result
            .config_string
            .contains("name=\"Motor.Drive.Torque\", identifier=\"23\""));
        assert!(!parse_result
            .config_string
            .contains("NamespaceUri"));
        assert_eq!(parse_result.measurement_name, "Motor");

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
            use_source_timestamp: false,
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
            use_source_timestamp: false,
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
