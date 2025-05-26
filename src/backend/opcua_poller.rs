use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::{error::TelegrafError, TelegrafConfig};
use opcua::{
    client::prelude::*,
    core::comms::url::is_opc_ua_binary_url,
    sync::*,
    types::{
        AttributeId, BrowseDescription, BrowseDescriptionResultMask, BrowseDirection,
        EndpointDescription, MessageSecurityMode, NodeClass, NodeId, ReferenceTypeId,
        UserTokenPolicy, Variant,
    },
};
use tokio;

#[derive(Debug, Clone)]
pub struct OpcUaNode {
    pub node_id: NodeId,
    pub browse_name: String,
    pub display_name: String,
    pub node_class: NodeClass,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub children: Vec<OpcUaNode>,
    pub selected: bool,
}

impl OpcUaNode {
    pub fn new(
        node_id: NodeId,
        browse_name: String,
        display_name: String,
        node_class: NodeClass,
    ) -> Self {
        Self {
            node_id,
            browse_name,
            display_name,
            node_class,
            data_type: None,
            description: None,
            children: Vec::new(),
            selected: false,
        }
    }
}

pub struct OpcUaPoller {
    config: TelegrafConfig,
    runtime: tokio::runtime::Runtime,
}

impl OpcUaPoller {
    pub fn new(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        // Validate the IP address in the config
        config.validate_ip()?;

        // Initialize logging
        opcua::console_logging::init();

        // Create a Tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            TelegrafError::OpcUaClientError(format!("Failed to create Tokio runtime: {}", e))
        })?;

        Ok(Self { config, runtime })
    }

    /// Get namespace information for XML files by connecting to the OPC UA server once
    /// Returns a map of file names to their corresponding namespace numbers
    pub fn get_namespace_info(
        &self,
        xml_files: &[String],
    ) -> Result<HashMap<String, u16>, TelegrafError> {
        // Connect to the OPC UA server using the configured IP
        let discovery_url = format!("opc.tcp://{}:4840/", self.config.ip);

        // Check if the server is reachable before attempting connection
        if let Err(e) = self.check_server_connectivity(&self.config.ip) {
            return Err(e);
        }

        // Get all namespace information from the server
        let namespaces = self.browse_server_namespaces(&discovery_url)?;

        // Process XML files to extract base names (without extension)
        let file_base_names: Vec<(String, String)> = xml_files
            .iter()
            .filter_map(|file_path| {
                let path = Path::new(file_path);
                let file_name = path.file_name()?.to_str()?.to_string();

                // Get the base name without extension
                let base_name = path.file_stem()?.to_str()?.to_string();

                Some((file_name, base_name))
            })
            .collect();

        // Match XML file names with namespace information
        let mut namespace_map = HashMap::new();

        for (file_name, base_name) in file_base_names {
            // Try to find a matching namespace
            for (namespace_index, namespace_name) in &namespaces {
                // Compare the namespace name with the XML base name (case insensitive)
                if namespace_name.to_lowercase() == base_name.to_lowercase() {
                    namespace_map.insert(file_name, *namespace_index);
                    break;
                }
            }
        }

        Ok(namespace_map)
    }

    /// Connect to OPC UA server and browse for namespace information
    fn browse_server_namespaces(
        &self,
        discovery_url: &str,
    ) -> Result<Vec<(u16, String)>, TelegrafError> {
        if !is_opc_ua_binary_url(discovery_url) {
            return Err(TelegrafError::OpcUaClientError(format!(
                "Not a valid OPC UA binary URL: {}",
                discovery_url
            )));
        }

        let mut client = ClientBuilder::new()
            .application_name("Telegraf OPC UA Client")
            .application_uri("urn:TelegrafOpcUaClient")
            .product_uri("urn:TelegrafOpcUaClient")
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(3)
            .client()
            .ok_or_else(|| {
                TelegrafError::ConfigError("Failed to create OPC UA client".to_string())
            })?;

        let endpoint: EndpointDescription = (
            discovery_url,
            "Basic256Sha256",
            MessageSecurityMode::Sign,
            UserTokenPolicy::anonymous(),
        )
            .into();

        // Use block_in_place for connections since they are blocking
        let session = {
            if !self.config.username.is_empty() && !self.config.password.is_empty() {
                client.connect_to_endpoint(
                    endpoint,
                    IdentityToken::UserName(
                        self.config.username.clone(),
                        self.config.password.clone(),
                    ),
                )
            } else {
                client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)
            }
        }
        .map_err(|e| {
            TelegrafError::OpcUaConnectionError(format!(
                "Failed to connect to OPC UA server: {}",
                e
            ))
        })?;

        let read_lock = session.read();

        let si_id = NodeId::new(3, "ServerInterfaces");
        let mut namespace_info = Vec::new();

        let browse_desc = BrowseDescription {
            node_id: si_id,
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::Organizes.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseDescriptionResultMask::all().bits() as u32,
        };

        let browse_results = read_lock.browse(&[browse_desc]);
        if let Ok(Some(ref results)) = browse_results {
            for result in results {
                if let Some(refs) = &result.references {
                    for reference in refs {
                        let namespace_index = reference.node_id.node_id.namespace;
                        // Convert UAString to String
                        let name = reference.browse_name.name.to_string();
                        namespace_info.push((namespace_index, name));
                    }
                }
            }
        }
        Ok(namespace_info)
    }

    /// Check if the OPC UA server is reachable before attempting a full connection
    fn check_server_connectivity(&self, ip: &str) -> Result<(), TelegrafError> {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;

        // Try to resolve the address
        let addr = format!("{}:4840", ip);
        let socket_addrs = addr.to_socket_addrs().map_err(|e| {
            TelegrafError::OpcUaConnectionError(format!(
                "Could not resolve OPC UA server address: {}",
                e
            ))
        })?;

        // Try connecting to the first resolved address with a timeout
        for socket_addr in socket_addrs {
            // Set a connect timeout of 3 seconds
            match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(3)) {
                Ok(_) => {
                    // Connection successful
                    return Ok(());
                }
                Err(e) => {
                    // Connection failed
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        return Err(TelegrafError::OpcUaTimeoutError(format!(
                            "Connection to OPC UA server at {} timed out",
                            ip
                        )));
                    } else {
                        return Err(TelegrafError::OpcUaConnectionError(format!(
                            "Failed to connect to OPC UA server: {}",
                            e
                        )));
                    }
                }
            }
        }

        // If we got here, we couldn't connect to any of the resolved addresses
        Err(TelegrafError::OpcUaConnectionError(format!(
            "Could not connect to OPC UA server at {}",
            ip
        )))
    }

    /// Connect to the OPC UA server and get a session
    fn connect_to_server(
        &self,
        discovery_url: &str,
    ) -> Result<Arc<RwLock<Session>>, TelegrafError> {
        if !is_opc_ua_binary_url(discovery_url) {
            return Err(TelegrafError::OpcUaClientError(format!(
                "Not a valid OPC UA binary URL: {}",
                discovery_url
            )));
        }

        let mut client = ClientBuilder::new()
            .application_name("Telegraf OPC UA Client")
            .application_uri("urn:TelegrafOpcUaClient")
            .product_uri("urn:TelegrafOpcUaClient")
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(3)
            .client()
            .ok_or_else(|| {
                TelegrafError::ConfigError("Failed to create OPC UA client".to_string())
            })?;

        let endpoint: EndpointDescription = (
            discovery_url,
            "Basic256Sha256",
            MessageSecurityMode::Sign,
            UserTokenPolicy::anonymous(),
        )
            .into();

        // Connect to server with appropriate credentials
        let session = {
            if !self.config.username.is_empty() && !self.config.password.is_empty() {
                client.connect_to_endpoint(
                    endpoint,
                    IdentityToken::UserName(
                        self.config.username.clone(),
                        self.config.password.clone(),
                    ),
                )
            } else {
                client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)
            }
        }
        .map_err(|e| {
            TelegrafError::OpcUaConnectionError(format!(
                "Failed to connect to OPC UA server: {}",
                e
            ))
        })?;

        Ok(session)
    }

    /// Browse all nodes in the OPC UA server starting from the root node
    pub fn browse_complete_structure(&self) -> Result<Vec<OpcUaNode>, TelegrafError> {
        // Connect to the OPC UA server using the configured IP
        let discovery_url = format!("opc.tcp://{}:4840/", self.config.ip);

        // Check if the server is reachable before attempting connection
        if let Err(e) = self.check_server_connectivity(&self.config.ip) {
            return Err(e);
        }

        // Connect to the server using our helper method
        let session = self.connect_to_server(&discovery_url)?;

        // Define a maximum browsing depth to prevent going too deep
        // Adjust this value based on your requirements
        const MAX_BROWSE_DEPTH: usize = 12;
        // Very large folders might not show all items since we're skipping continuation points
        // This prevents hangs in the OPC UA client when browsing large structures
        
        // Start browsing from the Objects folder with depth tracking
        let objects_folder = NodeId::objects_folder_id();
        self.browse_nodes(&session, &objects_folder, 0, MAX_BROWSE_DEPTH)
    }

    /// Recursively browse nodes starting from a given node
    fn browse_nodes(
        &self,
        session: &Arc<RwLock<Session>>,
        node_id: &NodeId,
        current_depth: usize,
        max_depth: usize,
    ) -> Result<Vec<OpcUaNode>, TelegrafError> {
        let mut nodes = Vec::new();

        // Create browse description for this node
        let browse_desc = BrowseDescription {
            node_id: node_id.clone(),
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::HierarchicalReferences.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseDescriptionResultMask::all().bits() as u32,
        };

        // Get a read lock on the session
        let session_read = session.read();

        // Execute initial browse request
        let browse_results = session_read.browse(&[browse_desc]);
        
        // Process the initial browse results
        if let Ok(Some(ref results)) = browse_results {
            for result in results {
                // Process references if present
                if let Some(refs) = &result.references {
                        // Process each reference
                        for reference in refs {
                            // Extract node info
                            let browse_name = reference.browse_name.name.to_string();
                            let display_name = reference.display_name.text.to_string();
                            let node_class = reference.node_class;
                            let child_node_id = reference.node_id.node_id.clone();

                            // Create the node - use clones to keep original values for later use
                            let mut node = OpcUaNode::new(
                                child_node_id.clone(),
                                browse_name.clone(),
                                display_name.clone(),
                                node_class,
                            );

                            // For variables, get the data type
                            if node_class == NodeClass::Variable {
                                // Create read value IDs for the attributes we want
                                let read_value_ids = [
                                    opcua::types::ReadValueId {
                                        node_id: child_node_id.clone(),
                                        attribute_id: AttributeId::DataType as u32,
                                        index_range: opcua::types::UAString::null(),
                                        data_encoding: opcua::types::QualifiedName::null(),
                                    },
                                    opcua::types::ReadValueId {
                                        node_id: child_node_id.clone(),
                                        attribute_id: AttributeId::Description as u32,
                                        index_range: opcua::types::UAString::null(),
                                        data_encoding: opcua::types::QualifiedName::null(),
                                    },
                                ];
                                // Read the attributes
                                let read_results = session_read.read(
                                    &read_value_ids,
                                    opcua::types::TimestampsToReturn::Neither,
                                    0.0, // max_age (0 = latest value)
                                );

                                if let Ok(read_results) = read_results {
                                    let mut attrs = std::collections::HashMap::new();
                                    for (i, result) in read_results.iter().enumerate() {
                                        if let Some(status) = &result.status {
                                            if status.is_good() {
                                                let attr_id = if i == 0 {
                                                    AttributeId::DataType
                                                } else {
                                                    AttributeId::Description
                                                };
                                                attrs.insert(attr_id, result.value.clone());
                                            }
                                        }
                                    }

                                    // Extract data type
                                    if let Some(data_type) = attrs.get(&AttributeId::DataType) {
                                        if let Some(value) = data_type {
                                            if let Variant::NodeId(data_type_id) = value {
                                                // Convert NodeId to string representation for data type
                                                // Use a more readable format for better display
                                                node.data_type = Some(format!("{:?}", data_type_id));
                                                
                                                // Handle special variable types that might need additional browsing
                                                if node.display_name.contains("Icon") {
                                                    // For variables that are icons, make the display name more descriptive
                                                    // by combining the browse name and data type
                                                    if !node.browse_name.is_empty() && node.browse_name != "%icon" {
                                                        node.display_name = format!("{} ({})", node.browse_name, data_type_id.to_string());
                                                    }
                                                }
                                        }
                                    }
                                }

                                    if let Some(desc) = attrs.get(&AttributeId::Description) {
                                        if let Some(value) = desc {
                                            if let Variant::LocalizedText(lt) = value {
                                                node.description = Some(lt.text.to_string());
                                            }
                                        }
                                    }
                                }
                            }

                            // Determine whether to browse children based on node class and depth
                            let should_browse_children = match node_class {
                                // Never browse methods
                                NodeClass::Method => false,
                                // Always browse objects and folders regardless of their name
                                NodeClass::Object | NodeClass::ObjectType => current_depth < max_depth,
                                // For variables, check if it might be a folder-like variable that should be browsed
                                NodeClass::Variable => {
                                    // Special case for known folder-like variables
                                    // DataBlocksGlobal and similar folders need special handling
                                    if node.display_name.contains("DataBlocks") || 
                                       node.display_name.contains("Global") ||
                                       node.browse_name.contains("DataBlocks") ||
                                       node.browse_name.contains("Global") {
                                        current_depth < max_depth
                                    } else {
                                        false
                                    }
                                },
                                // For other node types, browse if we haven't reached max depth
                                _ => current_depth < max_depth,
                            };

                            if should_browse_children {
                                // Recursively browse with incremented depth
                                let children = self.browse_nodes(&session, &child_node_id, current_depth + 1, max_depth)?;
                                node.children = children;
                            }

                            nodes.push(node);
                    }
                    
                }
                
                // Check for continuation point (if it exists)
                if !result.continuation_point.is_empty() {
                    // We have a continuation point, meaning there are more nodes to fetch
                    // Use browse_next to retrieve them
                    let mut continuation_points = vec![result.continuation_point.clone()];
                    let mut max_continuations = 5; // Limit to prevent infinite loops
                    
                    // Process continuation points with a safety limit
                    while !continuation_points.is_empty() && max_continuations > 0 {
                        // Call browse_next with the continuation points
                        // First parameter is release_continuation_points: bool
                        // Second parameter is continuation_points: &[ByteString]
                        let browse_next_result = session_read.browse_next(
                            false, // don't release continuation points yet
                            &continuation_points
                        );
                        
                        // Clear the current continuation points
                        continuation_points.clear();
                        
                        // Process the continuation point results
                        if let Ok(Some(next_results)) = browse_next_result {
                            for next_result in next_results {
                                // Process references if they exist
                                if let Some(refs) = &next_result.references {
                                    for reference in refs {
                                        // Extract node info (same as above)
                                        let browse_name = reference.browse_name.name.to_string();
                                        let display_name = reference.display_name.text.to_string();
                                        let node_class = reference.node_class;
                                        let child_node_id = reference.node_id.node_id.clone();
                                        
                                        // Create the node
                                        let mut node = OpcUaNode::new(
                                            child_node_id.clone(),
                                            browse_name.clone(),
                                            display_name.clone(),
                                            node_class,
                                        );
                                        
                                        // For variables, get the data type (simplified version)
                                        if node_class == NodeClass::Variable {
                                            // Here we could add variable-specific handling
                                            // But for now we'll skip detailed attribute reading to avoid hangs
                                        }
                                        
                                        // We'll skip recursive browsing for continuation point nodes
                                        // to avoid excessive depth and potential hangs
                                        nodes.push(node);
                                    }
                                }
                                
                                // Store the next continuation point if it exists
                                if !next_result.continuation_point.is_empty() {
                                    continuation_points.push(next_result.continuation_point.clone());
                                }
                            }
                        } else {
                            // Error or no results from browse_next, break the loop
                            break;
                        }
                        
                        // Decrement our safety counter
                        max_continuations -= 1;
                    }
                    
                    // Release any remaining continuation points
                    if !continuation_points.is_empty() {
                        let _ = session_read.browse_next(true, &continuation_points);
                    }
                }
            }
        }

        Ok(nodes)
    }
}
