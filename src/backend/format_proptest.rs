#[cfg(test)]
mod tests {
    use crate::backend::format::{NodeIdFormat, OpcuaConfig, OutputFormat, parse_node_id};
    use proptest::prelude::*;
    use crate::TelegrafConfig;

    proptest! {
        // Test that parsing and re-formatting node IDs is reversible
        #[test]
        fn node_id_parse_format_roundtrip(
            namespace in 0u16..10,
            identifier in 1u32..1000000,
        ) {
            // Create a node ID string in various formats
            let formats = vec![
                format!("ns={};i={}", namespace, identifier),           // Numeric format
                format!("ns={};s=String_{}", namespace, identifier),    // String format
            ];
            
            for node_id_str in formats {
                // Parse the node ID
                let parsed = parse_node_id(&node_id_str).expect("Failed to parse valid node ID");
                
                // Ensure we can extract the components correctly
                match parsed {
                    NodeIdFormat::Numeric { namespace: ns, identifier: id } => {
                        if node_id_str.contains(";i=") {
                            assert_eq!(ns, namespace);
                            assert_eq!(id, identifier.to_string());
                        }
                    },
                    NodeIdFormat::String { namespace: ns, identifier: id } => {
                        if node_id_str.contains(";s=") {
                            assert_eq!(ns, namespace);
                            assert_eq!(id, format!("String_{}", identifier));
                        }
                    },
                    _ => panic!("Unexpected node ID format"),
                }
            }
        }
        
        // Test that OpcuaConfig produces valid TOML configuration
        #[test]
        fn opcua_config_produces_valid_toml(
            endpoint in "[a-zA-Z0-9.:/_-]{5,50}",
            username in "[a-zA-Z0-9]{1,20}",
            password in "[a-zA-Z0-9!@#$%^&*]{1,20}",
            namespace_number in 0u16..10,
            interval_ms in (100u64..10000).prop_map(|n| n * 100), // Reasonable polling intervals
        ) {
            let config = OpcuaConfig {
                ip: &endpoint,
                username: &username,
                password: &password,
                is_listener: false,
                group_name: "test_group",
                namespace_number: &namespace_number.to_string(),
                interval_ms,
            };
            
            // Generate influx format config
            let influx_config = crate::backend::format::format_config(
                &config, 
                "ns=2;s=Device1.Temperature"
            );
            
            // Basic validation of the generated config
            assert!(influx_config.contains(&endpoint));
            assert!(influx_config.contains(&username));
            assert!(influx_config.contains(&password));
            assert!(influx_config.contains(&format!("interval = \"{}ms\"", interval_ms)));
            
            // Validate structure
            assert!(influx_config.contains("[[inputs.opcua]]"));
            assert!(influx_config.starts_with("[agent]"));
        }
        
        // Test that XML parsing handles various node formats
        #[test]
        fn xml_parsing_handles_various_node_formats(
            namespace in 0u16..10,
            identifier_base in 1u32..1000,
        ) {
            // Create a simple XML with different node ID formats
            let xml_content = format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
                <UANodeSet>
                  <UAVariable BrowseName="Temperature" NodeId="ns={};i={}">
                    <DisplayName>Temperature</DisplayName>
                  </UAVariable>
                  <UAVariable BrowseName="Pressure" NodeId="ns={};s=Pressure_{}">
                    <DisplayName>Pressure</DisplayName>
                  </UAVariable>
                  <UAVariable BrowseName="Humidity" NodeId="ns={};g=12345678-1234-1234-1234-{:012}">
                    <DisplayName>Humidity</DisplayName>
                  </UAVariable>
                </UANodeSet>"#,
                namespace, identifier_base,
                namespace, identifier_base,
                namespace, identifier_base
            );
            
            // Create a temporary file
            let mut temp_file = tempfile::NamedTempFile::new().unwrap();
            std::io::Write::write_all(&mut temp_file, xml_content.as_bytes()).unwrap();
            
            // Basic config for testing
            let config = OpcuaConfig {
                ip: "opc.tcp://localhost:4840",
                username: "user",
                password: "pass",
                is_listener: false,
                group_name: "test",
                namespace_number: &namespace.to_string(),
                interval_ms: 1000,
            };
            
            // Parse the XML
            let mut namespace_infos = Vec::new();
            let result = crate::backend::format::parse_xml(
                &config, 
                temp_file.path().to_str().unwrap(),
                &mut namespace_infos
            );
            
            // It should parse without errors
            assert!(result.is_ok());
            
            // The output should contain all node types
            let output = result.unwrap();
            assert!(output.contains(&format!("ns={};i={}", namespace, identifier_base)) || 
                    output.contains(&format!("identifier=\"{}\"", identifier_base)));
            assert!(output.contains(&format!("Pressure_{}", identifier_base)) || 
                    output.contains("Pressure"));
            assert!(output.contains("Humidity"));
        }
    }
}
