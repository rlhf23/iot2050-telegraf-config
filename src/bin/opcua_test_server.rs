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
    
    // Parse UAObject nodes to create folders
    let mut objects = HashMap::new();
    let mut object_references = HashMap::new();
    
    for node in doc.descendants() {
        if node.tag_name().name() == "UAObject" {
            if let Some(node_id_str) = node.attribute("NodeId") {
                if let Some(browse_name) = node.attribute("BrowseName") {
                    let display_name = node.children()
                        .find(|n| n.tag_name().name() == "DisplayName")
                        .and_then(|n| n.text())
                        .unwrap_or(browse_name);
                    
                    objects.insert(node_id_str.to_string(), (browse_name.to_string(), display_name.to_string()));
                    
                    // Parse references to understand hierarchy
                    if let Some(references) = node.children().find(|n| n.tag_name().name() == "References") {
                        let mut organizes_refs = Vec::new();
                        for ref_node in references.children() {
                            if ref_node.tag_name().name() == "Reference" {
                                if let Some(ref_type) = ref_node.attribute("ReferenceType") {
                                    if ref_type == "Organizes" && ref_node.attribute("IsForward").is_none() {
                                        // This is a parent reference (IsForward=false or not specified means parent)
                                        if let Some(target) = ref_node.text() {
                                            organizes_refs.push(target.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        if !organizes_refs.is_empty() {
                            object_references.insert(node_id_str.to_string(), organizes_refs);
                        }
                    }
                }
            }
        }
    }
    
    // Parse UAVariable nodes
    let mut variables = HashMap::new();
    for node in doc.descendants() {
        if node.tag_name().name() == "UAVariable" {
            if let Some(node_id_str) = node.attribute("NodeId") {
                if let Some(browse_name) = node.attribute("BrowseName") {
                    let display_name = node.children()
                        .find(|n| n.tag_name().name() == "DisplayName")
                        .and_then(|n| n.text())
                        .unwrap_or(browse_name);
                    
                    let data_type = node.attribute("DataType").unwrap_or("String");
                    
                    variables.insert(node_id_str.to_string(), (browse_name.to_string(), display_name.to_string(), data_type.to_string()));
                }
            }
        }
    }
    
    // Create the object hierarchy starting from Objects folder
    let mut created_objects = HashMap::new();
    created_objects.insert("i=85".to_string(), NodeId::objects_folder_id()); // Standard Objects folder
    
    // Create ServerInterfaces object
    if let Some((browse_name, display_name)) = objects.get("ns=1;s=ServerInterfaces") {
        let server_interfaces_id = {
            let mut address_space = address_space.write();
            address_space
                .add_folder(browse_name, display_name, &NodeId::objects_folder_id())
                .unwrap()
        };
        created_objects.insert("ns=1;s=ServerInterfaces".to_string(), server_interfaces_id);
    }
    
    // Create Sample_DB object under ServerInterfaces  
    if let Some((browse_name, display_name)) = objects.get("ns=2;i=1") {
        if let Some(server_interfaces_id) = created_objects.get("ns=1;s=ServerInterfaces") {
            let sample_db_id = {
                let mut address_space = address_space.write();
                address_space
                    .add_folder(browse_name, display_name, server_interfaces_id)
                    .unwrap()
            };
            created_objects.insert("ns=2;i=1".to_string(), sample_db_id);
        }
    }
    
    // Create variables under Sample_DB
    if let Some(sample_db_id) = created_objects.get("ns=2;i=1") {
        let mut variables_to_create = Vec::new();
        
        for (node_id_str, (browse_name, display_name, data_type)) in &variables {
            if node_id_str.starts_with("ns=2;i=") {
                let node_id = parse_node_id(node_id_str)?;
                let default_value = get_default_value_for_type(data_type);
                
                variables_to_create.push(Variable::new(&node_id, browse_name, display_name, default_value));
            }
        }
        
        if !variables_to_create.is_empty() {
            let mut address_space = address_space.write();
            let _ = address_space.add_variables(variables_to_create, sample_db_id);
        }
    }
    
    Ok(())
}

fn parse_node_id(node_id_str: &str) -> Result<NodeId, TelegrafError> {
    if let Some(rest) = node_id_str.strip_prefix("ns=") {
        if let Some((ns_str, id_part)) = rest.split_once(';') {
            let namespace: u16 = ns_str.parse()
                .map_err(|_| TelegrafError::ConfigError(format!("Invalid namespace in NodeId: {}", node_id_str)))?;
            
            if let Some(id_str) = id_part.strip_prefix("i=") {
                let id: u32 = id_str.parse()
                    .map_err(|_| TelegrafError::ConfigError(format!("Invalid numeric ID in NodeId: {}", node_id_str)))?;
                return Ok(NodeId::new(namespace, id));
            } else if let Some(id_str) = id_part.strip_prefix("s=") {
                return Ok(NodeId::new(namespace, id_str.to_string()));
            }
        }
    }
    
    Err(TelegrafError::ConfigError(format!("Unsupported NodeId format: {}", node_id_str)))
}

fn get_default_value_for_type(data_type: &str) -> opcua::types::Variant {
    match data_type {
        "REAL" => opcua::types::Variant::Float(0.0),
        "Boolean" => opcua::types::Variant::Boolean(false),
        "Int32" => opcua::types::Variant::Int32(0),
        "String" => opcua::types::Variant::String(UAString::from("")),
        _ => opcua::types::Variant::Float(0.0), // Default to Float for unknown types like REAL
    }
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
