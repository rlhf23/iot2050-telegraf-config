use std::path::Path;
use std::collections::HashMap;

use opcua::{
    client::prelude::*,
    core::comms::url::is_opc_ua_binary_url,
    types::{
        BrowseDescription, BrowseDescriptionResultMask, BrowseDirection,
        EndpointDescription, MessageSecurityMode, ReferenceTypeId, UserTokenPolicy,
    },
};
use tokio;
use crate::{TelegrafConfig, error::TelegrafError};

pub struct OpcUaPoller {
    config: TelegrafConfig,
    runtime: tokio::runtime::Runtime,
}

impl OpcUaPoller {
    pub fn new(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        // Validate the IP address in the config
        config.validate_ip()?;
        
        // Create a Tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create Tokio runtime: {}", e))
        })?;
        
        Ok(Self {
            config,
            runtime,
        })
    }
    
    /// Get namespace information for XML files by connecting to the OPC UA server once
    /// Returns a map of file names to their corresponding namespace numbers
    pub fn get_namespace_info(&self, xml_files: &[String]) -> Result<HashMap<String, u16>, TelegrafError> {
        self.runtime.block_on(async {
            // Connect to the OPC UA server using the configured IP
            let discovery_url = format!("opc.tcp://{}:4840/", self.config.ip);
            
            // Get all namespace information from the server
            let namespaces = self.browse_server_namespaces(&discovery_url).await?;
            
            // Process XML files to extract base names (without extension)
            let file_base_names: Vec<(String, String)> = xml_files.iter()
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
        })
    }

    /// Connect to OPC UA server and browse for namespace information
    async fn browse_server_namespaces(&self, discovery_url: &str) -> Result<Vec<(u16, String)>, TelegrafError> {
        if !is_opc_ua_binary_url(discovery_url) {
            return Err(TelegrafError::ValidationError(
                format!("Not a valid OPC UA binary URL: {}", discovery_url)
            ));
        }

        let mut client = ClientBuilder::new()
            .application_name("Telegraf OPC UA Client")
            .application_uri("urn:TelegrafOpcUaClient")
            .product_uri("urn:TelegrafOpcUaClient")
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(3)
            .client()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to create OPC UA client: {}", e)))?;

        let endpoint: EndpointDescription = (
            discovery_url,
            "Basic256Sha256",
            MessageSecurityMode::Sign,
            UserTokenPolicy::anonymous(),
        )
            .into();

        // Use block_in_place for connections since they are blocking
        let session = tokio::task::block_in_place(move || {
            if !self.config.username.is_empty() && !self.config.password.is_empty() {
                client.connect_to_endpoint(
                    endpoint,
                    IdentityToken::UserName(self.config.username.clone(), self.config.password.clone()),
                )
            } else {
                client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)
            }
        }).map_err(|e| TelegrafError::ConfigError(format!("Failed to connect to OPC UA server: {}", e)))?;

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
                        let name = reference.browse_name.name.clone();
                        namespace_info.push((namespace_index, name));
                    }
                }
            }
        }
        
        Ok(namespace_info)
    }
}
