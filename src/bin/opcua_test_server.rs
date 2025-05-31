use opcua::server::prelude::*;
use opcua::types::NodeId;

use sie_generate_config::error::TelegrafError;

fn main() -> Result<(), TelegrafError> {
    // Initialize logging
    opcua::console_logging::init();

    let address = "127.0.0.1";
    let port = 4840;

    // Create a new server instance for the thread using the same configuration
    let server_builder = ServerBuilder::new_sample()
        .application_name("OPC UA Test Server")
        .application_uri("urn:opcua-test-server")
        .product_uri("urn:opcua-test-server:product")
        .host_and_port(address, port);

    // Build the server
    let mut server = match server_builder.server() {
        Some(s) => s,
        _ => {
            return Err(TelegrafError::OpcUaClientError(
                "Failed to create server for thread".to_string(),
            ))
        }
    };
    let ns = {
        let address_space = server.address_space();
        let mut address_space = address_space.write();
        address_space
            .register_namespace("urn:opcua-test-server")
            .unwrap()
    };

    // Add some variables of our own
    add_example_variables(&mut server, ns);
    println!("Starting OPC UA test server at {}:{}", address, port);
    server.run();

    Ok(())
}

pub fn add_example_variables(server: &mut Server, ns: u16) {
    // These will be the node ids of the new variables
    let v1_node = NodeId::new(ns, "v1");
    let v2_node = NodeId::new(ns, "v2");
    let v3_node = NodeId::new(ns, "v3");
    let v4_node = NodeId::new(ns, "v4");

    let address_space = server.address_space();

    // The address space is guarded so obtain a lock to change it
    {
        let mut address_space = address_space.write();

        // Create a sample folder under objects folder
        let sample_folder_id = address_space
            .add_folder("Sample", "Sample", &NodeId::objects_folder_id())
            .unwrap();

        // Add some variables to our sample folder. Values will be overwritten by the timer
        let _ = address_space.add_variables(
            vec![
                Variable::new(&v1_node, "v1", "v1", 0_i32),
                Variable::new(&v2_node, "v2", "v2", false),
                Variable::new(&v3_node, "v3", "v3", UAString::from("")),
                Variable::new(&v4_node, "v4", "v4", 0f64),
            ],
            &sample_folder_id,
        );
    }
    {
        let mut address_space = address_space.write();
        let test_folder = address_space
            .add_folder("TestFolder", "TestFolder", &NodeId::objects_folder_id())
            .unwrap();
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
        let _ = address_space.add_variables(vec![var1, var2, var3], &test_folder);
    }
}
