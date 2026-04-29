use clap::Parser;
use opcua::server::prelude::*;
use opcua::types::NodeId;
use std::collections::HashMap;
use std::path::PathBuf;

use sie_generate_config::error::TelegrafError;

/// Simple OPC UA test server for integration testing
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address to bind to (default: 127.0.0.1)
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,

    /// Port to listen on (default: 4840)
    #[arg(short, long, default_value_t = 4840)]
    port: u16,

    /// Path to XML file with nodes to load (default: tests/sample_db.xml)
    #[arg(short, long, default_value = "tests/sample_db.xml")]
    nodes: PathBuf,
}

fn main() -> Result<(), TelegrafError> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging
    opcua::console_logging::init();

    let port = args.port;

    // Create a new server instance for the thread using the same configuration
    // new_sample() already creates sample keypair and endpoints including Basic256Sha256/Sign
    let server_builder = ServerBuilder::new_sample()
        .application_name("OPC UA Test Server")
        .application_uri("urn:opcua-test-server")
        .product_uri("urn:opcua-test-server:product")
        .host_and_port(&args.address, port);

    let address = args.address.clone(); // Clone for later use in println!

    // Build the server
    let mut server = match server_builder.server() {
        Some(s) => s,
        _ => {
            return Err(TelegrafError::OpcUaClientError(
                "Failed to create server for thread".to_string(),
            ))
        }
    };
    let _ns = {
        let address_space = server.address_space();
        let mut address_space = address_space.write();
        address_space
            .register_namespace("urn:opcua-test-server")
            .unwrap()
    };

    // Load and parse XML file to create nodes
    let mut any_loaded = false;
    match load_nodes_from_xml(&mut server, &args.nodes.to_string_lossy()) {
        Ok(_) => {
            println!("Successfully loaded nodes from {}", args.nodes.display());
            any_loaded = true;
        }
        Err(e) => {
            println!("Failed to load nodes from {}: {}", args.nodes.display(), e);
        }
    }

    if !any_loaded {
        println!("No nodes loaded from XML, adding example variables");
        add_example_variables(&mut server);
    }

    // Always add the ServerInterfaces structure at ns=3 so that
    // get_namespace_info() can browse it. This mimics the Siemens PLC
    // convention where ServerInterfaces lives at NodeId(3, "ServerInterfaces").
    add_server_interfaces_namespace(&mut server);

    // Set up Ctrl-C handler for clean shutdown
    ctrlc::set_handler(move || {
        println!("\nShutting down OPC UA server...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    // Run the server (this will block until Ctrl-C)
    println!("Starting OPC UA server at {}:{}", address, port);
    println!("OPC UA server running. Press Ctrl-C to stop.");
    server.run();

    println!("OPC UA server stopped");

    Ok(())
}

fn load_nodes_from_xml(server: &mut Server, xml_file_path: &str) -> Result<(), TelegrafError> {
    let xml_content =
        std::fs::read_to_string(xml_file_path).map_err(TelegrafError::IoError)?;

    let doc = roxmltree::Document::parse(&xml_content)
        .map_err(|e| TelegrafError::ConfigError(format!("Failed to parse XML: {}", e)))?;

    // Parse namespace URIs
    let mut namespaces = HashMap::new();
    if let Some(namespace_uris) = doc
        .descendants()
        .find(|n| n.tag_name().name() == "NamespaceUris")
    {
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
        for uri in namespaces.values() {
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
                    let display_name = node
                        .children()
                        .find(|n| n.tag_name().name() == "DisplayName")
                        .and_then(|n| n.text())
                        .unwrap_or(browse_name);

                    objects.insert(
                        node_id_str.to_string(),
                        (browse_name.to_string(), display_name.to_string()),
                    );

                    // Parse references to understand hierarchy
                    if let Some(references) = node
                        .children()
                        .find(|n| n.tag_name().name() == "References")
                    {
                        let mut organizes_refs = Vec::new();
                        for ref_node in references.children() {
                            if ref_node.tag_name().name() == "Reference" {
                                if let Some(ref_type) = ref_node.attribute("ReferenceType") {
                                    if ref_type == "Organizes"
                                        && ref_node.attribute("IsForward").is_none()
                                    {
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
                    let display_name = node
                        .children()
                        .find(|n| n.tag_name().name() == "DisplayName")
                        .and_then(|n| n.text())
                        .unwrap_or(browse_name);

                    let data_type = node.attribute("DataType").unwrap_or("String");

                    variables.insert(
                        node_id_str.to_string(),
                        (
                            browse_name.to_string(),
                            display_name.to_string(),
                            data_type.to_string(),
                        ),
                    );
                }
            }
        }
    }

    // Create the object hierarchy starting from Objects folder
    let mut created_objects = HashMap::new();
    created_objects.insert("i=85".to_string(), NodeId::objects_folder_id()); // Standard Objects folder

    // Create ServerInterfaces object if it doesn't exist
    let server_interfaces_id =
        if let Some(existing_id) = created_objects.get("ns=1;s=ServerInterfaces") {
            existing_id.clone()
        } else if let Some((browse_name, display_name)) = objects.get("ns=1;s=ServerInterfaces") {
            let server_interfaces_id = {
                let mut address_space = address_space.write();
                address_space
                    .add_folder(browse_name, display_name, &NodeId::objects_folder_id())
                    .unwrap()
            };
            created_objects.insert(
                "ns=1;s=ServerInterfaces".to_string(),
                server_interfaces_id.clone(),
            );
            server_interfaces_id
        } else {
            // Create default ServerInterfaces if not found in XML
            let server_interfaces_id = {
                let mut address_space = address_space.write();
                address_space
                    .add_folder(
                        "ServerInterfaces",
                        "ServerInterfaces",
                        &NodeId::objects_folder_id(),
                    )
                    .unwrap()
            };
            created_objects.insert(
                "ns=1;s=ServerInterfaces".to_string(),
                server_interfaces_id.clone(),
            );
            server_interfaces_id
        };

    // Create data block objects under ServerInterfaces
    for (object_node_id, (browse_name, display_name)) in &objects {
        if object_node_id.starts_with("ns=2;i=") {
            let data_block_id = {
                let mut address_space = address_space.write();
                address_space
                    .add_folder(browse_name, display_name, &server_interfaces_id)
                    .unwrap()
            };
            created_objects.insert(object_node_id.clone(), data_block_id);
        }
    }

    // Create variables under their respective data block objects
    for (object_node_id, object_id) in &created_objects {
        if object_node_id.starts_with("ns=2;i=") {
            let mut variables_to_create = Vec::new();

            for (node_id_str, (browse_name, display_name, data_type)) in &variables {
                if node_id_str.starts_with("ns=2;i=") {
                    let node_id = parse_node_id(node_id_str)?;
                    let default_value = get_default_value_for_type(data_type);

                    variables_to_create.push(Variable::new(
                        &node_id,
                        browse_name,
                        display_name,
                        default_value,
                    ));
                }
            }

            if !variables_to_create.is_empty() {
                let mut address_space = address_space.write();
                let _ = address_space.add_variables(variables_to_create, object_id);
                break; // Only add variables once per file
            }
        }
    }

    Ok(())
}

fn parse_node_id(node_id_str: &str) -> Result<NodeId, TelegrafError> {
    if let Some(rest) = node_id_str.strip_prefix("ns=") {
        if let Some((ns_str, id_part)) = rest.split_once(';') {
            let namespace: u16 = ns_str.parse().map_err(|_| {
                TelegrafError::ConfigError(format!("Invalid namespace in NodeId: {}", node_id_str))
            })?;

            if let Some(id_str) = id_part.strip_prefix("i=") {
                let id: u32 = id_str.parse().map_err(|_| {
                    TelegrafError::ConfigError(format!(
                        "Invalid numeric ID in NodeId: {}",
                        node_id_str
                    ))
                })?;
                return Ok(NodeId::new(namespace, id));
            } else if let Some(id_str) = id_part.strip_prefix("s=") {
                return Ok(NodeId::new(namespace, id_str.to_string()));
            }
        }
    }

    Err(TelegrafError::ConfigError(format!(
        "Unsupported NodeId format: {}",
        node_id_str
    )))
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

/// Add the ServerInterfaces folder at ns=3 with data block children,
/// mimicking the Siemens PLC convention that get_namespace_info() relies on.
/// The function ensures the namespace array has at least 4 entries (ns 0-3)
/// and creates:
///   - ServerInterfaces at NodeId(3, "ServerInterfaces"), organized by Objects
///   - Sample_DB data block under ServerInterfaces, with variables in ns=2
///
/// This is only added if the node doesn't already exist (e.g. from XML loading).
fn add_server_interfaces_namespace(server: &mut Server) {
    let address_space = server.address_space();

    // Ensure namespace indices 1, 2, 3 exist. ns=0 is always the standard
    // namespace. register_namespace returns the next available index but
    // will return an existing index if the URI is already registered.
    let uris = &[
        "urn:opcua-test-server",
        "http://www.siemens.com/simatic-s7-opcua",
        "http://Sample_DB",
    ];
    {
        let mut address_space = address_space.write();
        for uri in uris {
            let _ = address_space.register_namespace(uri);
        }
    }

    let si_node_id = NodeId::new(3u16, "ServerInterfaces");

    // Check if ServerInterfaces already exists at this NodeId
    {
        let address_space = address_space.read();
        if address_space.find_node(&si_node_id).is_some() {
            println!("ServerInterfaces folder already exists at ns=3;s=ServerInterfaces, skipping creation");
            return;
        }
    }

    // Create ServerInterfaces folder at ns=3;s=ServerInterfaces under Objects
    let ok = {
        let mut address_space = address_space.write();
        address_space.add_folder_with_id(
            &si_node_id,
            "ServerInterfaces",
            "ServerInterfaces",
            &NodeId::objects_folder_id(),
        )
    };
    if ok {
        println!("Created ServerInterfaces folder at ns=3;s=ServerInterfaces");
    } else {
        println!("ServerInterfaces folder creation failed");
        return;
    }

    // Create a data block folder "Sample_DB" organized by ServerInterfaces.
    // Its browse name must match the XML filename (case-insensitive) so that
    // get_namespace_info() can map "sample_db.xml" → namespace index.
    // We place the data block in ns=2 so the reference's namespace index
    // points to the application namespace.
    let db_node_id = NodeId::new(2u16, "Sample_DB");
    let db_ok = {
        let mut address_space = address_space.write();
        address_space.add_folder_with_id(
            &db_node_id,
            "Sample_DB",
            "Sample_DB",
            &si_node_id,
        )
    };
    if db_ok {
        println!("Created Sample_DB data block at ns=2;s=Sample_DB under ServerInterfaces");
    } else {
        println!("Sample_DB data block already exists or creation failed");
    }

    // Add some variables under the Sample_DB data block in ns=2
    let vars = vec![
        Variable::new(&NodeId::new(2u16, "SI_Real_1"), "SI_Real_1", "SI_Real_1", 0.0f32),
        Variable::new(&NodeId::new(2u16, "SI_Real_2"), "SI_Real_2", "SI_Real_2", 0.0f32),
        Variable::new(&NodeId::new(2u16, "SI_Real_3"), "SI_Real_3", "SI_Real_3", 0.0f32),
    ];
    {
        let mut address_space = address_space.write();
        let results = address_space.add_variables(vars, &db_node_id);
        for (i, ok) in results.iter().enumerate() {
            if !ok {
                println!("Failed to add variable {} under Sample_DB", i);
            }
        }
    }
}

pub fn add_example_variables(server: &mut Server) {
    let address_space = server.address_space();
    let ns = {
        let mut address_space = address_space.write();
        address_space
            .register_namespace("urn:opcua-example-variables")
            .unwrap()
    };

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
