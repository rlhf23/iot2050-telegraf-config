use opcua::server::prelude::*;
use opcua::types::NodeId;
use std::collections::HashMap;

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

    // Load and parse XML to create nodes
    match load_nodes_from_xml(&mut server) {
        Ok(_) => println!("Successfully loaded nodes from XML"),
        Err(e) => {
            println!("Failed to load nodes from XML: {}. Using fallback nodes.", e);
            add_example_variables(&mut server, ns);
        }
    }
    println!("Starting OPC UA test server at {}:{}", address, port);
    server.run();

    Ok(())
}

fn load_nodes_from_xml(server: &mut Server) -> Result<(), TelegrafError> {
    let xml_content = std::fs::read_to_string("tests/sample_db.xml")
        .map_err(|e| TelegrafError::IoError(e))?;
    
    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| TelegrafError::ConfigError(format!("Failed to parse XML: {}", e)))?;
    
    // Parse namespace URIs
    let mut namespaces = HashMap::new();
    if let Some(namespace_uris) = doc.descendants().find(|n| n.tag_name().name() == "NamespaceUris") {
        for (index, uri_node) in namespace_uris.children().enumerate() {
            if uri_node.tag_name().name() == "Uri" {
                if let Some(uri) = uri_node.text() {
                    namespaces.insert(index + 1, uri.to_string());
                }
            }
        }
    }
    
    // Register namespaces and get their IDs
    let mut namespace_ids = HashMap::new();
    let address_space = server.address_space();
    {
        let mut address_space = address_space.write();
        for (_, uri) in &namespaces {
            let ns_id = address_space.register_namespace(uri).unwrap();
            namespace_ids.insert(uri.clone(), ns_id);
        }
    }
    
    // Create ServerInterfaces folder
    let server_interfaces_id = {
        let mut address_space = address_space.write();
        address_space
            .add_folder("ServerInterfaces", "ServerInterfaces", &NodeId::objects_folder_id())
            .unwrap()
    };
    
    // Find Sample_DB namespace ID
    let sample_db_ns = namespace_ids.get("http://Sample_DB")
        .copied()
        .unwrap_or(2);
    
    // Create Sample_DB folder under ServerInterfaces
    let sample_db_id = {
        let mut address_space = address_space.write();
        address_space
            .add_folder("Sample_DB", "Sample_DB", &server_interfaces_id)
            .unwrap()
    };
    
    // Create REAL variables under Sample_DB
    {
        let mut address_space = address_space.write();
        let variables = vec![
            Variable::new(&NodeId::new(sample_db_ns, 2u32), "Real_1", "Real_1", 0.0f32),
            Variable::new(&NodeId::new(sample_db_ns, 3u32), "Real_2", "Real_2", 0.0f32),
            Variable::new(&NodeId::new(sample_db_ns, 4u32), "Real_3", "Real_3", 0.0f32),
            Variable::new(&NodeId::new(sample_db_ns, 5u32), "Real_4", "Real_4", 0.0f32),
            Variable::new(&NodeId::new(sample_db_ns, 6u32), "Real_5", "Real_5", 0.0f32),
        ];
        let _ = address_space.add_variables(variables, &sample_db_id);
    }
    
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
    {
        let mut address_space = address_space.write();
        let test_folder = address_space
            .add_folder(
                "SampleInterface",
                "SampleInterface",
                &NodeId::objects_folder_id(),
            )
            .unwrap();
        // Create integer variable
        let var1 = Variable::new(&NodeId::new(ns, 10), "IntVar", "Integer Variable", 42_i32);

        // Create string variable
        let var2 = Variable::new(
            &NodeId::new(ns, 11),
            "StringVar",
            "String Variable",
            "Hello OPC UA!",
        );

        // Create boolean variable
        let var3 = Variable::new(&NodeId::new(ns, 12), "BoolVar", "Boolean Variable", true);

        // Add all variables to the test folder
        let _ = address_space.add_variables(vec![var1, var2, var3], &test_folder);
    }
}
