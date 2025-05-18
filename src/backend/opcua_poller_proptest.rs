#[cfg(test)]
mod tests {
    use crate::error::TelegrafError;
    use proptest::prelude::*;

    proptest! {
        // Test endpoint URL parsing
        #[test]
        fn endpoint_url_parsing_handles_valid_inputs(
            ip in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
            port in 1024u16..65535,
        ) {
            let endpoint = format!("opc.tcp://{}:{}/", ip, port);
            
            // Check that OPC UA URL validation works as expected
            let is_valid = opcua::core::comms::url::is_opc_ua_binary_url(&endpoint);
            assert!(is_valid, "URL {} should be recognized as valid OPC UA binary URL", endpoint);
            
            // Test with optional path components
            let endpoint_with_path = format!("opc.tcp://{}:{}/UA/Server", ip, port);
            let is_valid_with_path = opcua::core::comms::url::is_opc_ua_binary_url(&endpoint_with_path);
            assert!(is_valid_with_path, "URL {} should be recognized as valid OPC UA binary URL", endpoint_with_path);
            
            // Test that invalid schemas are rejected
            let invalid_endpoint = format!("http://{}:{}/", ip, port);
            let is_invalid = !opcua::core::comms::url::is_opc_ua_binary_url(&invalid_endpoint);
            assert!(is_invalid, "URL {} should be recognized as invalid OPC UA binary URL", invalid_endpoint);
        }
        
        // Test node ID validation
        #[test]
        fn node_id_validation_handles_various_inputs(
            namespace in 0u16..10,
            identifier in 1u32..1000000,
        ) {
            // Helper function to check if a node ID string is valid
            fn validate_node_id(node_id: &str) -> Result<(), TelegrafError> {
                if node_id.starts_with("ns=") && 
                (node_id.contains(";i=") || node_id.contains(";s=") || node_id.contains(";g=")) {
                    Ok(())
                } else {
                    Err(TelegrafError::OpcUaConnectionError(
                        format!("Invalid node ID format: {}", node_id)
                    ))
                }
            }
            
            // Test numeric format
            let numeric_node_id = format!("ns={};i={}", namespace, identifier);
            assert!(validate_node_id(&numeric_node_id).is_ok());
            
            // Test string format
            let string_node_id = format!("ns={};s=String_{}", namespace, identifier);
            assert!(validate_node_id(&string_node_id).is_ok());
            
            // Test GUID format
            let guid_node_id = format!("ns={};g=12345678-1234-1234-1234-{:012}", namespace, identifier);
            assert!(validate_node_id(&guid_node_id).is_ok());
            
            // Test invalid formats
            let invalid_node_id = format!("ns={};x={}", namespace, identifier); // Invalid type
            assert!(validate_node_id(&invalid_node_id).is_err());
            
            let missing_ns_node_id = format!("i={}", identifier); // Missing namespace
            assert!(validate_node_id(&missing_ns_node_id).is_err());
        }
        
        // Test parsing of OPC UA discovery URL
        #[test]
        fn discovery_url_construction_produces_valid_urls(
            ip in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
            port in 1024u16..65535,
        ) {
            // Standard discovery URL format
            let discovery_url = format!("opc.tcp://{}:{}/", ip, port);
            assert!(opcua::core::comms::url::is_opc_ua_binary_url(&discovery_url));
            
            // Discovery URL with discovery endpoint
            let discovery_url_with_endpoint = format!("opc.tcp://{}:{}/discovery", ip, port);
            assert!(opcua::core::comms::url::is_opc_ua_binary_url(&discovery_url_with_endpoint));
        }
    }
}
