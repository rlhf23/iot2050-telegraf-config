use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};

use clap::{Arg, Command};
use sie_generate_config::backend::opcua_test_server::{ping_server, OpcUaTestServer};
use sie_generate_config::error::TelegrafError;

fn main() -> Result<(), TelegrafError> {
    // Initialize logging
    opcua::console_logging::init();

    // Parse command-line arguments
    let matches = Command::new("OPC UA Test Server")
        .version("1.0.0")
        .about("A simple OPC UA test server for testing applications")
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .help("The network address to bind to")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("The port number to bind to")
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

    println!("Starting OPC UA test server at {}:{}", address, port);

    // Create a new test server
    let mut server = match OpcUaTestServer::new(address, port) {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Failed to create OPC UA test server: {}", e);
            exit(1);
        }
    };

    // Initialize the server with test nodes
    if let Err(e) = server.init() {
        eprintln!("Failed to initialize OPC UA test server: {}", e);
        exit(1);
    }

    // Create a flag for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up Ctrl+C handler for graceful shutdown
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Start the server
    if let Err(e) = server.start() {
        eprintln!("Failed to start OPC UA test server: {}", e);
        exit(1);
    }

    // Verify the server is running by pinging it
    let endpoint_url = server.endpoint_url();
    println!("Server started at endpoint: {}", endpoint_url);

    // Wait for the server to fully initialize
    // The errors about discovery server connection are normal and can be ignored
    println!("Waiting for server to initialize...");
    thread::sleep(Duration::from_secs(1));

    // Try to ping the server
    if ping_server(endpoint_url) {
        println!("Server is reachable and ready to accept connections");
    } else {
        eprintln!("Warning: Server might not be reachable");
    }

    println!("Press Ctrl+C to stop the server");

    // Keep the server running until Ctrl+C
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));
    }

    println!("OPC UA test server shutting down");
    Ok(())
}
