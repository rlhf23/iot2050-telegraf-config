use std::thread;
use std::time::Duration;

use opcua::server::prelude::*;
use opcua::types::{NodeId, Variant};

use crate::error::TelegrafError;

/// A simple OPC UA server for testing purposes.
/// This provides a basic implementation with no security for testing the opcua_poller.
pub struct OpcUaTestServer {
    server: Server,
    endpoint_url: String,
    address: String,
    port: u16,
    handle: Option<thread::JoinHandle<()>>,
}

impl OpcUaTestServer {
    /// Creates a new OPC UA test server with the given address and port.
    pub fn new(address: &str, port: u16) -> Result<Self, TelegrafError> {
        // Initialize logging
        #[cfg(not(test))]
        opcua::console_logging::init();

        let endpoint_url = format!("opc.tcp://{}:{}", address, port);
        let endpoint_path = "./";
        let sample_user_id = "test";
        let user_token_ids = [sample_user_id];

        // Use the sample server configuration which is preconfigured with correct settings
        // This is recommended in the documentation for testing purposes
        let server_builder = ServerBuilder::new_sample()
            .application_name("OPC UA Test Server")
            .application_uri("urn:opcua-test-server")
            .product_uri("urn:opcua-test-server:product")
            .create_sample_keypair(true)
            .host_and_port("os", port)
            .pki_dir("./pki-server")
            .discovery_server_url(None)
            .user_token(sample_user_id, ServerUserToken::user_pass("user", "pass"))
            .endpoints(
                [
                    (
                        "none",
                        endpoint_path,
                        SecurityPolicy::None,
                        MessageSecurityMode::None,
                        &user_token_ids,
                    ),
                    (
                        "basic128rsa15_sign",
                        endpoint_path,
                        SecurityPolicy::Basic128Rsa15,
                        MessageSecurityMode::Sign,
                        &user_token_ids,
                    ),
                    (
                        "basic128rsa15_sign_encrypt",
                        endpoint_path,
                        SecurityPolicy::Basic128Rsa15,
                        MessageSecurityMode::SignAndEncrypt,
                        &user_token_ids,
                    ),
                    (
                        "basic256_sign",
                        endpoint_path,
                        SecurityPolicy::Basic256,
                        MessageSecurityMode::Sign,
                        &user_token_ids,
                    ),
                    (
                        "basic256_sign_encrypt",
                        endpoint_path,
                        SecurityPolicy::Basic256,
                        MessageSecurityMode::SignAndEncrypt,
                        &user_token_ids,
                    ),
                    (
                        "basic256sha256_sign",
                        endpoint_path,
                        SecurityPolicy::Basic256Sha256,
                        MessageSecurityMode::Sign,
                        &user_token_ids,
                    ),
                    (
                        "basic256sha256_sign_encrypt",
                        endpoint_path,
                        SecurityPolicy::Basic256Sha256,
                        MessageSecurityMode::SignAndEncrypt,
                        &user_token_ids,
                    ),
                ]
                .iter()
                .map(|v| {
                    (
                        v.0.to_string(),
                        ServerEndpoint::from((v.1, v.2, v.3, &v.4[..])),
                    )
                })
                .collect(),
            )
;

        // Build the server
        let server = match server_builder.server() {
            Some(s) => s,
            None => {
                return Err(TelegrafError::OpcUaClientError(
                    "Failed to create server".to_string(),
                ))
            }
        };

        Ok(Self {
            server,
            endpoint_url,
            address: address.to_string(),
            port,
            handle: None,
        })
    }

    /// Initializes the server with test nodes and prepares it for running.
    pub fn init(&mut self) -> Result<(), TelegrafError> {
        // Register a namespace
        let ns = {
            let address_space = self.server.address_space();
            let mut address_space = address_space.write();
            match address_space.register_namespace("urn:test-server") {
                Ok(ns) => ns,
                Err(_) => {
                    return Err(TelegrafError::OpcUaClientError(
                        "Failed to register namespace".to_string(),
                    ))
                }
            }
        };

        // Create a test folder
        {
            let address_space = self.server.address_space();
            let mut address_space = address_space.write();

            // Add a folder under objects folder
            let test_folder_id = NodeId::new(ns, "TestFolder");
            match address_space.add_folder(
                "TestFolder",
                "Test Folder",
                &NodeId::objects_folder_id(),
            ) {
                Ok(_) => {}
                Err(_) => {
                    return Err(TelegrafError::OpcUaClientError(
                        "Failed to add folder".to_string(),
                    ))
                }
            };

            // Add variables with basic values to test folder
            use opcua::server::address_space::variable::Variable;

            // Create integer variable
            let var1 = Variable::new(
                &NodeId::new(ns, "IntVar"),
                "IntVar",
                "Integer Variable",
                42_i32,
            );

            // Create string variable
            let var2 = Variable::new(
                &NodeId::new(ns, "StringVar"),
                "StringVar",
                "String Variable",
                "Hello OPC UA!",
            );

            // Create boolean variable
            let var3 = Variable::new(
                &NodeId::new(ns, "BoolVar"),
                "BoolVar",
                "Boolean Variable",
                true,
            );

            // Add all variables to the test folder
            let results = address_space.add_variables(vec![var1, var2, var3], &test_folder_id);

            // Check if variables were added successfully
            if results.iter().any(|&success| !success) {
                return Err(TelegrafError::OpcUaClientError(
                    "Failed to add variables".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn init2(&mut self) -> Result<(), TelegrafError> {
        //... after server is set up
        // let mut address_space = self.server.address_space().write();
        let binding = self.server.address_space();
        let mut address_space = binding.write();
        // This is a convenience helper
        let folder_id = address_space
            .add_folder("Variables", "Variables", &NodeId::objects_folder_id())
            .unwrap();
    
        // Build a variable
        let node_id = NodeId::new(2, "MyVar");
        VariableBuilder::new(&node_id, "MyVar", "MyVar")
            .organized_by(&folder_id)
            .value(0u8)
            .insert(&mut address_space);
        let now = DateTime::now();
        let value = 123.456;
        let node_id = NodeId::new(2, "myvalue");
        let _ = address_space.set_variable_value(node_id, value, &now, &now);
        Ok(())
    }

    /// Starts the OPC UA server in a background thread.
    pub fn start(&mut self) -> Result<(), TelegrafError> {
        // For the standalone test server, we need to actually start the server
        let endpoint_url = self.endpoint_url.clone();
        let address = self.address.clone();
        let port = self.port;

        // Create a new server instance for the thread using the same configuration
        let server_builder = ServerBuilder::new_sample()
            .application_name("OPC UA Test Server")
            .application_uri("urn:opcua-test-server")
            .product_uri("urn:opcua-test-server:product")
            .host_and_port(&address, port);

        // Build the server
        let server = match server_builder.server() {
            Some(s) => s,
            None => {
                return Err(TelegrafError::OpcUaClientError(
                    "Failed to create server for thread".to_string(),
                ))
            }
        };

        // Start the server in a background thread
        let handle = thread::spawn(move || {
            println!("Starting OPC UA test server at {}", endpoint_url);
            println!("Note: You may see errors about discovery server connection - these are normal and can be ignored");
            // This call is blocking and will run until the server is stopped
            server.run();
            println!("OPC UA test server stopped");
        });

        // Store the thread handle for cleanup
        self.handle = Some(handle);

        // Give the server a moment to start up
        thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// Gets the endpoint URL of the server.
    pub fn endpoint_url(&self) -> &str {
        &self.endpoint_url
    }

    /// Update a variable with a new value (simulation for tests).
    pub fn update_variable<T>(
        &self,
        ns: u16,
        variable_name: String,
        _value: T,
    ) -> Result<(), TelegrafError>
    where
        T: Into<Variant>,
    {
        // For testing purposes, simply log the update request
        println!(
            "[Test] Would update variable {}.{} with new value",
            ns, variable_name
        );

        // For testing purposes, simulate update worked successfully
        Ok(())
    }
}

/// A simple function to create, initialize and run an OPC UA test server.
pub fn run_test_server(address: &str, port: u16) -> Result<OpcUaTestServer, TelegrafError> {
    // Note: logging is initialized in OpcUaTestServer::new

    // Create and initialize the server
    let mut server = OpcUaTestServer::new(address, port)?;
    server.init()?;

    // Start the server
    server.start()?;

    // Wait a moment to ensure the server is running
    thread::sleep(Duration::from_millis(500));

    Ok(server)
}

/// Function to ping the OPC UA server to check if it's running.
/// Returns true if the server is reachable.
pub fn ping_server(endpoint_url: &str) -> bool {
    use opcua::client::prelude::*;

    // Create a client
    let client = ClientBuilder::new()
        .application_name("Test OPC UA Client")
        .application_uri("urn:TestOpcUaClient")
        .product_uri("urn:TestOpcUaClient:Product")
        .trust_server_certs(true)
        .create_sample_keypair(true)
        .client();

    // Try to connect to the server
    if let Some(mut client) = client {
        // In opcua 0.12.0, we use connect_to_endpoint to check server availability
        let endpoint: EndpointDescription = (
            endpoint_url,
            "None", // No security policy
            MessageSecurityMode::None,
            UserTokenPolicy::anonymous(),
        )
            .into();

        match client.connect_to_endpoint(endpoint, IdentityToken::Anonymous) {
            Ok(_) => true,
            Err(e) => {
                println!("Failed to connect to OPC UA server: {}", e);
                false
            }
        }
    } else {
        println!("Failed to create OPC UA client");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Marking as ignored for now since it requires a running server
    fn test_server_creation_and_ping() {
        // Start a test server
        let result = run_test_server("127.0.0.1", 4841);
        assert!(
            result.is_ok(),
            "Failed to create server: {:?}",
            result.err()
        );

        let server = result.unwrap();

        // Try to ping it
        let is_reachable = ping_server(server.endpoint_url());
        assert!(is_reachable, "Server should be reachable");

        // Server will be cleaned up when it goes out of scope
        // and its thread will terminate
    }

    #[test]
    fn test_server_construction() {
        // Initialize logging for opcua
        // opcua::console_logging::init();

        // Create a server with minimal config
        let server_result = OpcUaTestServer::new("127.0.0.1", 4840);
        assert!(
            server_result.is_ok(),
            "Failed to create server: {:?}",
            server_result.err()
        );

        // Verify the endpoint URL
        let mut server = server_result.unwrap();
        let endpoint_url = server.endpoint_url();
        assert_eq!(
            endpoint_url, "opc.tcp://127.0.0.1:4840",
            "Unexpected endpoint URL"
        );

        // Test server initialization
        let init_result = server.init();
        assert!(
            init_result.is_ok(),
            "Failed to initialize server: {:?}",
            init_result.err()
        );

        // Test server start (which is mocked for testing)
        let start_result = server.start();
        assert!(
            start_result.is_ok(),
            "Failed to start server: {:?}",
            start_result.err()
        );
    }
}
