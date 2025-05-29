use std::process::exit;

use clap::{Arg, Command};
use opcua::client::prelude::*;
use opcua::types::{NodeId, Variant, MessageSecurityMode, ReadValueId, TimestampsToReturn};
use sie_generate_config::error::TelegrafError;

fn main() -> Result<(), TelegrafError> {
    // Initialize logging
    opcua::console_logging::init();

    // Parse command-line arguments
    let matches = Command::new("OPC UA Client Test")
        .version("1.0.0")
        .about("A test client to verify an OPC UA server is running correctly")
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .help("The server address to connect to")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("The server port to connect to")
                .default_value("4840"),
        )
        .get_matches();

    // Get the address and port from the command-line arguments
    let address = matches.get_one::<String>("address").unwrap();
    let port = matches
        .get_one::<String>("port")
        .unwrap()
        .parse::<u16>()
        .unwrap_or_else(|_| {
            eprintln!("Invalid port number");
            exit(1);
        });

    // Create the endpoint URL
    let endpoint_url = format!("opc.tcp://{}:{}", address, port);
    println!("Connecting to OPC UA server at {}", endpoint_url);

    // Create a client (following the pattern in ping_server)
    let client = ClientBuilder::new()
        .application_name("OPC UA Client Test")
        .application_uri("urn:opcua-client-test")
        .product_uri("urn:opcua-client-test:product")
        .trust_server_certs(true)
        .create_sample_keypair(true)
        .client();
    
    if let Some(mut client) = client {
        // Create the endpoint description
        let endpoint: EndpointDescription = (
            endpoint_url.as_str(),
            "None",
            MessageSecurityMode::None,
            UserTokenPolicy::anonymous(),
        )
            .into();
        
        // Connect to the server
        println!("Attempting to connect to the server...");
        match client.connect_to_endpoint(endpoint, IdentityToken::Anonymous) {
            Ok(session) => {
                println!("Successfully connected to the server!");
                
                // Read test variables
                println!("Testing read access to server variables...");
                
                let test_variables = [
                    ("IntVar", "Integer test variable"),
                    ("StringVar", "String test variable"),
                    ("BoolVar", "Boolean test variable"),
                ];
                
                // Try to read each test variable
                let namespace_index = 1; // Assuming the test namespace was registered with index 1
                let mut success_count = 0;
                
                for (var_name, description) in &test_variables {
                    // Read the variable directly here without a separate function
                    let node_id = NodeId::new(namespace_index, *var_name);
                    
                    // Create a read request
                    let read_id = ReadValueId {
                        node_id,
                        attribute_id: opcua::types::AttributeId::Value as u32,
                        index_range: opcua::types::UAString::null(),
                        data_encoding: opcua::types::QualifiedName::null(),
                    };
                    
                    // First, acquire a read lock on the session
                    let session_guard = session.read();
                    
                    // Now use the session through the read guard
                    let result = match session_guard.read(&[read_id], TimestampsToReturn::Both, 0.0) {
                        Ok(response) => {
                            if response.is_empty() {
                                Err("Empty response from server".to_string())
                            } else {
                                let data_value = &response[0];
                                if let Some(status) = data_value.status {
                                    if !status.is_good() {
                                        Err(format!("Read failed with status: {:?}", status))
                                    } else {
                                        match &data_value.value {
                                            Some(value) => Ok(value.clone()),
                                            None => Err("Variable has no value".to_string()),
                                        }
                                    }
                                } else {
                                    match &data_value.value {
                                        Some(value) => Ok(value.clone()),
                                        None => Err("Variable has no value".to_string()),
                                    }
                                }
                            }
                        },
                        Err(e) => Err(format!("Failed to read variable: {}", e)),
                    };
                    
                    match result {
                        Ok(value) => {
                            println!("✅ Successfully read {}: {}", description, format_variant(&value));
                            success_count += 1;
                        }
                        Err(e) => {
                            println!("❌ Failed to read {}: {}", description, e);
                        }
                    }
                }
                
                println!("\nTest Summary:");
                println!("Successfully read {}/{} test variables", success_count, test_variables.len());
                
                if success_count == test_variables.len() {
                    println!("🎉 All tests passed! The OPC UA server is functioning correctly.");
                } else {
                    println!("⚠️  Some tests failed. The OPC UA server might not be configured correctly.");
                }
            },
            Err(e) => {
                eprintln!("Failed to connect to the server: {}", e);
                exit(1);
            }
        }
    } else {
        eprintln!("Failed to create OPC UA client");
        exit(1);
    }
    
    Ok(())
}



/// Format a variant value as a string
fn format_variant(variant: &Variant) -> String {
    match variant {
        Variant::Boolean(v) => format!("Boolean: {}", v),
        Variant::SByte(v) => format!("SByte: {}", v),
        Variant::Byte(v) => format!("Byte: {}", v),
        Variant::Int16(v) => format!("Int16: {}", v),
        Variant::UInt16(v) => format!("UInt16: {}", v),
        Variant::Int32(v) => format!("Int32: {}", v),
        Variant::UInt32(v) => format!("UInt32: {}", v),
        Variant::Int64(v) => format!("Int64: {}", v),
        Variant::UInt64(v) => format!("UInt64: {}", v),
        Variant::Float(v) => format!("Float: {}", v),
        Variant::Double(v) => format!("Double: {}", v),
        Variant::String(v) => format!("String: {}", v),
        _ => format!("{:?}", variant),
    }
}
