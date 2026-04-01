use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::{error::TelegrafError, OpcUaConnectionConfig, TelegrafConfig};
use opcua::{
    client::prelude::*,
    core::comms::url::is_opc_ua_binary_url,
    sync::*,
    types::{
        AttributeId, BrowseDescription, BrowseDescriptionResultMask, BrowseDirection, ByteString,
        EndpointDescription, MessageSecurityMode, NodeClass, NodeId, ReferenceTypeId,
        UserTokenPolicy, Variant,
    },
};

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
    pub children_loaded: bool,   // Whether children have been loaded
    pub has_more_children: bool, // Whether there are more children via continuation points
    pub continuation_point: Option<ByteString>, // Store continuation point for lazy loading
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
            children_loaded: false,
            has_more_children: false,
            continuation_point: None,
        }
    }
}

pub struct OpcUaPoller {
    config: OpcUaConnectionConfig,
}

impl OpcUaPoller {
    /// Create a new OpcUaPoller with connection config
    pub fn new(config: OpcUaConnectionConfig) -> Result<Self, TelegrafError> {
        // Validate the IP address
        Self::validate_ip(&config.ip)?;

        Ok(Self { config })
    }

    /// Create from TelegrafConfig (for backward compatibility during migration)
    pub fn from_telegraf_config(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        Self::new(config.connection_config())
    }

    /// Validate IP address format
    fn validate_ip(ip: &str) -> Result<(), TelegrafError> {
        // Extract just the IP part (before the port)
        let ip_part = ip.split(':').next().unwrap_or(ip);

        // Check if it's a valid IPv4 address
        if ip_part.parse::<std::net::Ipv4Addr>().is_err() {
            return Err(TelegrafError::ConfigError(format!(
                "Invalid IP address format: '{}'. Must be a valid IPv4 address (e.g., 192.168.1.1 or 192.168.1.1:4840)",
                ip
            )));
        }

        Ok(())
    }

    /// Original new() method kept for backward compatibility
    #[deprecated(note = "Use new() with OpcUaConnectionConfig or from_telegraf_config() instead")]
    pub fn new_legacy(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        Self::from_telegraf_config(config)
    }

    /// Get namespace information for XML files by connecting to the OPC UA server once
    /// Returns a map of file names to their corresponding namespace numbers
    pub fn get_namespace_info(
        &self,
        xml_files: &[String],
    ) -> Result<HashMap<String, u16>, TelegrafError> {
        // Connect to the OPC UA server using the configured IP
        let discovery_url = format!("opc.tcp://{}/", self.config.ip);

        // Check if the server is reachable before attempting connection
        if let Err(e) = self.check_server_connectivity(&self.config.ip) {
            return Err(e);
        }

        // Get all namespace information from the server
        let namespaces = self.browse_server_namespaces(&discovery_url)?;

        // Check if we got any namespaces from the server
        if namespaces.is_empty() {
            return Err(TelegrafError::OpcUaClientError(
                "No namespaces found on OPC-UA server. The server may not have ServerInterfaces configured, or the browse operation failed. Check server logs for details.".to_string()
            ));
        }

        // Log the namespaces we found for debugging
        eprintln!("Found {} namespaces from OPC-UA server:", namespaces.len());
        for (idx, name) in &namespaces {
            eprintln!("  Namespace {}: {}", idx, name);
        }

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

        eprintln!(
            "Looking for matches with {} XML files:",
            file_base_names.len()
        );
        for (file_name, base_name) in &file_base_names {
            eprintln!("  File: {} (base: {})", file_name, base_name);
        }

        // Match XML file names with namespace information
        let mut namespace_map = HashMap::new();

        for (file_name, base_name) in file_base_names {
            // Try to find a matching namespace
            let mut found = false;
            for (namespace_index, namespace_name) in &namespaces {
                // Compare the namespace name with the XML base name (case insensitive)
                if namespace_name.to_lowercase() == base_name.to_lowercase() {
                    eprintln!(
                        "  ✓ Matched '{}' to namespace {} ({})",
                        file_name, namespace_index, namespace_name
                    );
                    namespace_map.insert(file_name.clone(), *namespace_index);
                    found = true;
                    break;
                }
            }
            if !found {
                eprintln!(
                    "  ✗ No match found for '{}' (base: {})",
                    file_name, base_name
                );
            }
        }

        if namespace_map.is_empty() {
            return Err(TelegrafError::OpcUaClientError(
                format!(
                    "Could not match any XML files to server namespaces. Found {} namespaces on server but none matched the {} uploaded file(s). Check that your XML file names match the namespace names on the server.",
                    namespaces.len(),
                    xml_files.len()
                )
            ));
        }

        eprintln!(
            "Successfully mapped {} file(s) to namespaces",
            namespace_map.len()
        );
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

        eprintln!("Browsing ServerInterfaces node for namespace information...");

        let browse_results = read_lock.browse(&[browse_desc]);
        match browse_results {
            Ok(Some(ref results)) => {
                eprintln!("Browse succeeded, processing {} result(s)", results.len());
                for result in results {
                    if let Some(refs) = &result.references {
                        eprintln!("  Found {} reference(s)", refs.len());
                        for reference in refs {
                            let namespace_index = reference.node_id.node_id.namespace;
                            // Convert UAString to String
                            let name = reference.browse_name.name.to_string();
                            eprintln!("    - Namespace {}: {}", namespace_index, name);
                            namespace_info.push((namespace_index, name));
                        }
                    } else {
                        eprintln!("  Browse result has no references");
                    }
                }
            }
            Ok(None) => {
                eprintln!("Browse returned None - no results found");
                return Err(TelegrafError::OpcUaClientError(
                    "Browse operation returned no results. The ServerInterfaces node may not exist on this server.".to_string()
                ));
            }
            Err(e) => {
                eprintln!("Browse operation failed: {}", e);
                return Err(TelegrafError::OpcUaClientError(format!(
                    "Failed to browse ServerInterfaces node: {}",
                    e
                )));
            }
        }

        Ok(namespace_info)
    }

    /// Check if the OPC UA server is reachable before attempting a full connection
    fn check_server_connectivity(&self, ip: &str) -> Result<(), TelegrafError> {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;

        // If IP already contains port, use it directly; otherwise add default port
        let addr = if ip.contains(':') {
            ip.to_string()
        } else {
            // If no port specified, use default OPC UA port
            format!("{}:4840", ip)
        };

        let mut socket_addrs = addr.to_socket_addrs().map_err(|e| {
            TelegrafError::OpcUaConnectionError(format!(
                "Could not resolve OPC UA server address: {}",
                e
            ))
        })?;

        // Try connecting to the first resolved address with a timeout
        if let Some(socket_addr) = socket_addrs.next() {
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

        // Connect to server with appropriate credentials
        let session = {
            if !self.config.username.is_empty() && !self.config.password.is_empty() {
                let endpoint: EndpointDescription = (
                    discovery_url,
                    "Basic256Sha256",
                    MessageSecurityMode::Sign,
                    UserTokenPolicy::anonymous(),
                )
                    .into();
                client.connect_to_endpoint(
                    endpoint,
                    IdentityToken::UserName(
                        self.config.username.clone(),
                        self.config.password.clone(),
                    ),
                )
            } else {
                let endpoint: EndpointDescription = (
                    discovery_url,
                    "Basic256Sha256",
                    MessageSecurityMode::Sign,
                    UserTokenPolicy::anonymous(),
                )
                    .into();
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
    /// This now uses lazy loading to avoid performance issues with large node structures
    ///
    /// Note: The returned nodes maintain a reference to the session that created them.
    /// The session will be automatically closed when all nodes are dropped.
    pub fn browse_complete_structure(&self) -> Result<Vec<OpcUaNode>, TelegrafError> {
        let discovery_url = format!("opc.tcp://{}/", self.config.ip);

        // Check server connectivity first
        self.check_server_connectivity(&self.config.ip)?;

        // Connect to server
        let session = self.connect_to_server(&discovery_url)?;

        // Define max browse depth
        const MAX_BROWSE_DEPTH: usize = 12;

        // Start browsing from the Objects folder
        let objects_folder = NodeId::objects_folder_id();

        // Perform the browsing operation
        // The session is kept alive as long as the returned nodes are in use
        self.browse_nodes(&session, &objects_folder, 0, MAX_BROWSE_DEPTH, false)
    }

    /// Browse nodes starting from a given node with support for lazy loading
    fn browse_nodes(
        &self,
        session: &Arc<RwLock<Session>>,
        node_id: &NodeId,
        current_depth: usize,
        max_depth: usize,
        load_children: bool, // Whether to load children
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
                                                if !node.browse_name.is_empty()
                                                    && node.browse_name != "%icon"
                                                {
                                                    node.display_name = format!(
                                                        "{} ({})",
                                                        node.browse_name,
                                                        data_type_id.to_string()
                                                    );
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

                        // Determine whether node could have children based on node class and depth
                        let should_browse_children = match node_class {
                            // Never browse methods
                            NodeClass::Method => false,
                            // Always browse objects and folders regardless of their name
                            NodeClass::Object | NodeClass::ObjectType => current_depth < max_depth,
                            // For variables, check if it might be a folder-like variable that should be browsed
                            NodeClass::Variable => {
                                // Special case for known folder-like variables
                                // DataBlocksGlobal and similar folders need special handling
                                if node.display_name.contains("DataBlocks")
                                    || node.display_name.contains("Global")
                                    || node.browse_name.contains("DataBlocks")
                                    || node.browse_name.contains("Global")
                                {
                                    current_depth < max_depth
                                } else {
                                    false
                                }
                            }
                            // For other node types, browse if we haven't reached max depth
                            _ => current_depth < max_depth,
                        };

                        // Mark as having children if it should
                        if should_browse_children {
                            // Only browse children if explicitly requested
                            if load_children {
                                // Recursively browse with incremented depth
                                let children = self.browse_nodes(
                                    session,
                                    &child_node_id,
                                    current_depth + 1,
                                    max_depth,
                                    true,
                                )?;
                                node.children = children;
                                node.children_loaded = true;
                            }
                        }

                        nodes.push(node);
                    }
                }

                // Check for continuation point (if it exists)
                if !result.continuation_point.is_empty() {
                    // For lazy loading, process continuation points
                    // We'll process all continuation points immediately rather than creating placeholder nodes
                    let mut continuation_points = vec![result.continuation_point.clone()];
                    let mut max_continuations = 5; // Limit to prevent infinite loops

                    // Process continuation points with a safety limit
                    while !continuation_points.is_empty() && max_continuations > 0 {
                        // Call browse_next with the continuation points
                        let browse_next_result = session_read.browse_next(
                            false, // don't release continuation points yet
                            &continuation_points,
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
                                        let node = OpcUaNode::new(
                                            child_node_id.clone(),
                                            browse_name.clone(),
                                            display_name.clone(),
                                            node_class,
                                        );

                                        // For variables, get the data type (simplified version)
                                        if node_class == NodeClass::Variable {
                                            // Here we could add variable-specific handling
                                        }

                                        // Determine whether node could have children
                                        let should_browse_children = match node_class {
                                            NodeClass::Method => false,
                                            NodeClass::Object | NodeClass::ObjectType => {
                                                current_depth < max_depth
                                            }
                                            NodeClass::Variable => {
                                                if node.display_name.contains("DataBlocks")
                                                    || node.display_name.contains("Global")
                                                    || node.browse_name.contains("DataBlocks")
                                                    || node.browse_name.contains("Global")
                                                {
                                                    current_depth < max_depth
                                                } else {
                                                    false
                                                }
                                            }
                                            _ => current_depth < max_depth,
                                        };

                                        // Only set the flag, don't browse
                                        if should_browse_children {
                                            // Set the flag to indicate it has children (will be loaded on demand)
                                            // But don't recursively browse them now
                                        }

                                        nodes.push(node);
                                    }
                                }

                                // Handle the next continuation point
                                if !next_result.continuation_point.is_empty() {
                                    continuation_points
                                        .push(next_result.continuation_point.clone());
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

    /// Load nodes using a continuation point (used for lazy loading)
    fn browse_nodes_on_demand(
        &self,
        session: &Arc<RwLock<Session>>,
        continuation_point: ByteString,
        current_depth: usize,
        max_depth: usize,
    ) -> Result<Vec<OpcUaNode>, TelegrafError> {
        let mut continuation_points = vec![continuation_point];
        let mut max_continuations = 5; // Limit to prevent infinite loops

        let mut nodes = Vec::new();

        // Get a read lock on the session
        let session_read = session.read();

        while !continuation_points.is_empty() && max_continuations > 0 {
            let browse_next_result = session_read.browse_next(false, &continuation_points);

            continuation_points.clear();

            if let Ok(Some(next_results)) = browse_next_result {
                for next_result in next_results {
                    if let Some(refs) = &next_result.references {
                        for reference in refs {
                            let browse_name = reference.browse_name.name.to_string();
                            let display_name = reference.display_name.text.to_string();
                            let node_class = reference.node_class;
                            let child_node_id = reference.node_id.node_id.clone();

                            let node = OpcUaNode::new(
                                child_node_id.clone(),
                                browse_name.clone(),
                                display_name.clone(),
                                node_class,
                            );

                            // Determine whether node could have children based on node class and depth
                            let should_browse_children = match node_class {
                                // Never browse methods
                                NodeClass::Method => false,
                                // Always browse objects and folders regardless of their name
                                NodeClass::Object | NodeClass::ObjectType => {
                                    current_depth < max_depth
                                }
                                // For variables, check if it might be a folder-like variable that should be browsed
                                NodeClass::Variable => {
                                    // Special case for known folder-like variables
                                    // DataBlocksGlobal and similar folders need special handling
                                    if node.display_name.contains("DataBlocks")
                                        || node.display_name.contains("Global")
                                        || node.browse_name.contains("DataBlocks")
                                        || node.browse_name.contains("Global")
                                    {
                                        current_depth < max_depth
                                    } else {
                                        false
                                    }
                                }
                                // For other node types, browse if we haven't reached max depth
                                _ => current_depth < max_depth,
                            };

                            // Just mark node as potentially having children, don't browse now
                            if should_browse_children {
                                // Not loading children now
                            }

                            nodes.push(node);
                        }
                    }

                    if !next_result.continuation_point.is_empty() {
                        // Create a placeholder node for the next continuation point
                        let mut more_node = OpcUaNode::new(
                            NodeId::null(),
                            String::from("__more_items__"),
                            String::from("Load more items..."),
                            NodeClass::Object,
                        );
                        more_node.has_more_children = true;
                        more_node.continuation_point = Some(next_result.continuation_point.clone());
                        nodes.push(more_node);
                        break; // Exit early, we'll load more on demand
                    }
                }
            } else {
                break;
            }

            max_continuations -= 1;
        }

        if !continuation_points.is_empty() {
            let _ = session_read.browse_next(true, &continuation_points);
        }

        Ok(nodes)
    }

    /// Public method to load a node's children on demand
    /// This is used by the GUI to implement lazy loading
    ///
    /// Note: The returned nodes maintain a reference to the session that created them.
    /// The session will be automatically closed when all nodes are dropped.
    pub fn load_node_children(
        &self,
        node: &OpcUaNode,
        depth: usize,
    ) -> Result<Vec<OpcUaNode>, TelegrafError> {
        // Connect to the OPC UA server
        let discovery_url = format!("opc.tcp://{}/", self.config.ip);

        // Check if the server is reachable
        if let Err(e) = self.check_server_connectivity(&self.config.ip) {
            return Err(e);
        }

        // Connect to the server
        let session = self.connect_to_server(&discovery_url)?;

        // Define a maximum browsing depth to prevent going too deep
        const MAX_BROWSE_DEPTH: usize = 12;

        // Return the appropriate result based on whether we're continuing a browse or starting a new one
        // The session is kept alive as long as the returned nodes are in use
        if node.has_more_children && node.continuation_point.is_some() {
            // Load more items using the continuation point
            self.browse_nodes_on_demand(
                &session,
                node.continuation_point.clone().unwrap(),
                depth,
                MAX_BROWSE_DEPTH,
            )
        } else {
            // Otherwise, browse the node's children
            self.browse_nodes(&session, &node.node_id, depth, MAX_BROWSE_DEPTH, true)
        }
    }

    /// Helper function to check if a node is a placeholder for loading more items
    pub fn is_load_more_node(node: &OpcUaNode) -> bool {
        node.has_more_children && node.browse_name == "__more_items__"
    }

    /// Discover all namespaces and variables from ServerInterfaces
    /// This browses the PLC's ServerInterfaces node and collects all variables
    /// from each namespace, returning them as SelectedOpcUaNode objects.
    pub fn discover_all_namespaces(&self) -> Result<crate::DiscoveredData, TelegrafError> {
        let discovery_url = format!("opc.tcp://{}/", self.config.ip);

        // Check if the server is reachable
        if let Err(e) = self.check_server_connectivity(&self.config.ip) {
            return Err(e);
        }

        // Connect to the server
        let session = self.connect_to_server(&discovery_url)?;

        // Browse ServerInterfaces to get namespaces
        let server_interfaces_id = NodeId::new(3, "ServerInterfaces");

        eprintln!("Browsing ServerInterfaces node for namespaces...");

        let browse_desc = BrowseDescription {
            node_id: server_interfaces_id,
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::Organizes.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseDescriptionResultMask::all().bits() as u32,
        };

        let session_read = session.read();
        let browse_results = session_read.browse(&[browse_desc]);

        let mut namespaces = Vec::new();
        let mut all_variables = Vec::new();
        let mut namespace_names = std::collections::HashMap::new();

        match browse_results {
            Ok(Some(results)) => {
                for result in results {
                    if let Some(refs) = &result.references {
                        for reference in refs {
                            let namespace_index = reference.node_id.node_id.namespace;
                            let namespace_name = reference.browse_name.name.to_string();

                            eprintln!("  Found namespace {}: {}", namespace_index, namespace_name);

                            namespace_names.insert(namespace_index, namespace_name.clone());

                            // Browse all variables in this namespace
                            let namespace_root = reference.node_id.node_id.clone();
                            let variables = self.browse_namespace_variables(
                                &session,
                                &namespace_root,
                                &namespace_name,
                                namespace_index,
                            )?;

                            let variable_count = variables.len();

                            namespaces.push(crate::DiscoveredNamespace {
                                index: namespace_index,
                                name: namespace_name,
                                variable_count,
                            });

                            all_variables.extend(variables);
                        }
                    }
                }
            }
            Ok(None) => {
                return Err(TelegrafError::OpcUaClientError(
                    "No namespaces found under ServerInterfaces".to_string(),
                ));
            }
            Err(e) => {
                return Err(TelegrafError::OpcUaClientError(format!(
                    "Failed to browse ServerInterfaces: {}",
                    e
                )));
            }
        }

        eprintln!(
            "Discovered {} namespaces with {} total variables",
            namespaces.len(),
            all_variables.len()
        );

        Ok(crate::DiscoveredData {
            namespaces,
            variables: all_variables,
            namespace_names,
            plc_name: None,
        })
    }

    /// Browse all variables under a namespace root node
    fn browse_namespace_variables(
        &self,
        session: &Arc<RwLock<Session>>,
        namespace_root: &NodeId,
        namespace_name: &str,
        namespace_index: u16,
    ) -> Result<Vec<crate::SelectedOpcUaNode>, TelegrafError> {
        let mut variables = Vec::new();
        const MAX_DEPTH: usize = 10;

        self.collect_variables_recursive(
            session,
            namespace_root,
            namespace_name,
            "", // Start with empty path prefix at root
            namespace_index,
            0,
            MAX_DEPTH,
            &mut variables,
        )?;

        Ok(variables)
    }

    /// Build a full hierarchical name from path prefix and browse name
    fn build_full_name(path_prefix: &str, browse_name: &str) -> String {
        if path_prefix.is_empty() {
            browse_name.to_string()
        } else {
            format!("{}.{}", path_prefix, browse_name)
        }
    }

    /// Recursively collect all variable nodes with hierarchical path tracking
    fn collect_variables_recursive(
        &self,
        session: &Arc<RwLock<Session>>,
        node_id: &NodeId,
        namespace_name: &str,
        path_prefix: &str,
        namespace_index: u16,
        current_depth: usize,
        max_depth: usize,
        variables: &mut Vec<crate::SelectedOpcUaNode>,
    ) -> Result<(), TelegrafError> {
        if current_depth >= max_depth {
            return Ok(());
        }

        let browse_desc = BrowseDescription {
            node_id: node_id.clone(),
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::HierarchicalReferences.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseDescriptionResultMask::all().bits() as u32,
        };

        // Collect nodes to process: (NodeId, browse_name) for objects to recurse into
        let mut objects_to_browse: Vec<(NodeId, String)> = Vec::new();

        {
            let session_read = session.read();
            let browse_results = session_read.browse(&[browse_desc]);

            // Process initial browse results
            if let Ok(Some(results)) = browse_results {
                for result in &results {
                    if let Some(refs) = &result.references {
                        for reference in refs {
                            let child_node_id = reference.node_id.node_id.clone();
                            let browse_name = reference.browse_name.name.to_string();
                            let display_name = reference.display_name.text.to_string();
                            let node_class = reference.node_class;

                            match node_class {
                                NodeClass::Variable => {
                                    // Build full hierarchical name
                                    let full_name =
                                        Self::build_full_name(path_prefix, &browse_name);

                                    variables.push(crate::SelectedOpcUaNode {
                                        node_id: child_node_id,
                                        namespace: namespace_index,
                                        browse_name: full_name.clone(),
                                        display_name: full_name,
                                        measurement_name: namespace_name.to_string(),
                                        interval_ms: 1000,
                                        folder_name: Some(namespace_name.to_string()),
                                    });
                                }
                                NodeClass::Object | NodeClass::ObjectType => {
                                    // Store objects with their browse_name for path tracking
                                    objects_to_browse.push((child_node_id, browse_name));
                                }
                                _ => {}
                            }
                        }
                    }

                    // Handle continuation points (paged results from server)
                    if !result.continuation_point.is_empty() {
                        let mut continuation_points = vec![result.continuation_point.clone()];
                        let mut max_continuations = 5; // Limit to prevent infinite loops

                        while !continuation_points.is_empty() && max_continuations > 0 {
                            let browse_next_result = session_read.browse_next(
                                false, // don't release continuation points yet
                                &continuation_points,
                            );

                            continuation_points.clear();

                            if let Ok(Some(next_results)) = browse_next_result {
                                for next_result in next_results {
                                    if let Some(refs) = &next_result.references {
                                        for reference in refs {
                                            let child_node_id = reference.node_id.node_id.clone();
                                            let browse_name =
                                                reference.browse_name.name.to_string();
                                            let display_name =
                                                reference.display_name.text.to_string();
                                            let node_class = reference.node_class;

                                            match node_class {
                                                NodeClass::Variable => {
                                                    // Build full hierarchical name
                                                    let full_name = Self::build_full_name(
                                                        path_prefix,
                                                        &browse_name,
                                                    );

                                                    variables.push(crate::SelectedOpcUaNode {
                                                        node_id: child_node_id,
                                                        namespace: namespace_index,
                                                        browse_name: full_name.clone(),
                                                        display_name: full_name,
                                                        measurement_name: namespace_name
                                                            .to_string(),
                                                        interval_ms: 1000,
                                                        folder_name: Some(
                                                            namespace_name.to_string(),
                                                        ),
                                                    });
                                                }
                                                NodeClass::Object | NodeClass::ObjectType => {
                                                    // Store objects with their browse_name for path tracking
                                                    objects_to_browse
                                                        .push((child_node_id, browse_name));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }

                                    // Queue next continuation point if present
                                    if !next_result.continuation_point.is_empty() {
                                        continuation_points
                                            .push(next_result.continuation_point.clone());
                                    }
                                }
                            } else {
                                // Error or no more results, stop processing
                                break;
                            }

                            max_continuations -= 1;
                        }

                        // Release any remaining continuation points
                        if !continuation_points.is_empty() {
                            let _ = session_read.browse_next(true, &continuation_points);
                        }
                    }
                }
            }
        }
        // Session lock is released here

        // Recursively browse objects (DataBlocks, folders, etc.) with accumulated path
        for (object_node_id, object_browse_name) in objects_to_browse {
            // Build the path prefix for this object's children
            let child_path = Self::build_full_name(path_prefix, &object_browse_name);

            self.collect_variables_recursive(
                session,
                &object_node_id,
                namespace_name,
                &child_path,
                namespace_index,
                current_depth + 1,
                max_depth,
                variables,
            )?;
        }

        Ok(())
    }
}

// Node manipulation utilities
impl OpcUaNode {
    /// Helper function to determine if a node is a folder-like node
    pub fn is_folder_node(&self) -> bool {
        self.node_class == opcua::types::NodeClass::Object
            || self.node_class == opcua::types::NodeClass::ObjectType
            || (self.node_class == opcua::types::NodeClass::Variable
                && (self.display_name.contains("DataBlocks")
                    || self.display_name.contains("Global")
                    || self.browse_name.contains("DataBlocks")
                    || self.browse_name.contains("Global")))
    }

    /// Deselect all children of a node recursively
    pub fn deselect_children(&mut self) {
        for child in &mut self.children {
            child.selected = false;
            child.deselect_children();
        }
    }

    /// Collect only the variable nodes that are selected (for individual selection)
    pub fn collect_selected_variable_nodes(nodes: &[OpcUaNode], result: &mut Vec<OpcUaNode>) {
        for node in nodes {
            if node.selected
                && node.node_class == opcua::types::NodeClass::Variable
                && !node.is_folder_node()
            {
                result.push(node.clone());
            }

            // Still check children regardless of parent selection state
            Self::collect_selected_variable_nodes(&node.children, result);
        }
    }

    /// Collect only the folder nodes that are selected (for folder-based configuration)
    pub fn collect_selected_folder_nodes(nodes: &[OpcUaNode], result: &mut Vec<OpcUaNode>) {
        for node in nodes {
            if node.selected && node.is_folder_node() {
                result.push(node.clone());
            }

            // Check children recursively
            Self::collect_selected_folder_nodes(&node.children, result);
        }
    }

    /// Collect all variable nodes within a folder, regardless of their selection state
    pub fn collect_all_variables_in_folder(folder: &OpcUaNode, result: &mut Vec<OpcUaNode>) {
        for child in &folder.children {
            if child.node_class == opcua::types::NodeClass::Variable && !child.is_folder_node() {
                result.push(child.clone());
            }

            // Recursively collect from subfolders
            if child.is_folder_node() {
                Self::collect_all_variables_in_folder(child, result);
            }
        }
    }

    /// Original method kept for backward compatibility
    pub fn collect_selected_nodes(nodes: &[OpcUaNode], result: &mut Vec<OpcUaNode>) {
        for node in nodes {
            if node.selected {
                result.push(node.clone());
            }

            // Still check children regardless of parent selection state
            Self::collect_selected_nodes(&node.children, result);
        }
    }

    /// Convert selected nodes to SelectedOpcUaNode objects for configuration
    /// Returns a vector of SelectedOpcUaNode that can be added to the configuration
    pub fn convert_selected_nodes_to_config(nodes: &[OpcUaNode]) -> Vec<crate::SelectedOpcUaNode> {
        let mut result = Vec::new();

        // 1. Collect all selected folders
        let mut selected_folders = Vec::new();
        Self::collect_selected_folder_nodes(nodes, &mut selected_folders);

        // 2. For each selected folder, collect all variable nodes inside
        for folder in selected_folders {
            // Create a group name from the folder display name
            let group_name = folder.display_name.clone();

            // Collect all variables from the folder recursively
            let mut folder_variables = Vec::new();
            Self::collect_all_variables_in_folder(&folder, &mut folder_variables);

            // Add each variable with the folder name for grouping
            for var_node in folder_variables {
                let selected_node = crate::SelectedOpcUaNode {
                    node_id: var_node.node_id.clone(),
                    namespace: var_node.node_id.namespace,
                    browse_name: var_node.browse_name.clone(),
                    display_name: var_node.display_name.clone(),
                    measurement_name: var_node
                        .display_name
                        .clone()
                        .replace(" ", "_")
                        .to_lowercase(),
                    interval_ms: 1000,                     // Default interval
                    folder_name: Some(group_name.clone()), // Set the folder name for grouping
                };

                result.push(selected_node);
            }
        }

        // 3. Add individually selected variables (not from folders)
        let mut selected_variables = Vec::new();
        Self::collect_selected_variable_nodes(nodes, &mut selected_variables);

        for var_node in selected_variables {
            let selected_node = crate::SelectedOpcUaNode {
                node_id: var_node.node_id.clone(),
                namespace: var_node.node_id.namespace,
                browse_name: var_node.browse_name.clone(),
                display_name: var_node.display_name.clone(),
                measurement_name: var_node
                    .display_name
                    .clone()
                    .replace(" ", "_")
                    .to_lowercase(),
                interval_ms: 1000, // Default interval
                folder_name: None, // Not part of a folder group
            };

            result.push(selected_node);
        }

        result
    }
}
