use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use opcua::{
    client::prelude::*,
    core::comms::url::is_opc_ua_binary_url,
    crypto::SecurityPolicy,
    types::{
        BrowseDescription, BrowseDescriptionResultMask, BrowseDirection, ByteString,
        EndpointDescription, MessageSecurityMode, ObjectId, ReferenceTypeId, UserTokenPolicy,
    },
};
use tokio;

const DEFAULT_DISCOVERY_URL: &str = "opc.tcp://192.168.10.11:4840/";

#[tokio::main]
async fn main() {
    // Initialize logging
    opcua::console_logging::init();

    match print_server_info(DEFAULT_DISCOVERY_URL.to_string()).await {
        Ok(_) => println!("Browsing completed successfully."),
        Err(e) => println!("Error occurred: {:?}", e),
    }
}

async fn print_server_info(discovery_url: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovery URL: {}", discovery_url);

    if !is_opc_ua_binary_url(&discovery_url) {
        return Ok(()); // Not a valid OPC UA binary URL
    }

    let mut client = ClientBuilder::new()
        .application_name("My First Client")
        .application_uri("urn:MyFirstClient")
        .product_uri("urn:MyFirstClient")
        .create_sample_keypair(true)
        .trust_server_certs(true)
        .session_retry_limit(3)
        .client()
        .unwrap();

    let endpoint: EndpointDescription = (
        DEFAULT_DISCOVERY_URL,
        "Basic256Sha256",
        MessageSecurityMode::Sign,
        UserTokenPolicy::anonymous(),
    )
        .into();

    // Use block_in_place for connections since they are blocking
    let session = tokio::task::block_in_place(move || {
        client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)
        // client.connect_to_endpoint(
        //     endpoint,
        //     IdentityToken::UserName("Username".to_string(), "Password".to_string()),
        // )
    })?;

    let read_lock = session.read();

    let si_id = NodeId::new(3, "ServerInterfaces");

    let browse_desc = BrowseDescription {
        // node_id: ObjectId::RootFolder.into(),
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
                    let mut node_info = Vec::new();
                    let namespace_index = reference.node_id.node_id.namespace;
                    let name = reference.browse_name.name.clone();
                    println!("({}, {})", namespace_index.clone(), name.clone());
                    node_info.push((namespace_index, name));
                }
            }
        }
    }
    Ok(())
}
