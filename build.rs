use dotenv::dotenv;
use std::env;

fn main() {
    // Load .env file - don't fail if it doesn't exist (using ok())
    dotenv().ok();

    // List of environment variables to process with default values
    let vars = vec![
        ("DEFAULT_IP", "192.168.1.1:4840"),
        ("DEFAULT_USERNAME", "user"),
        ("DEFAULT_PASSWORD", "pass"),
        ("DEFAULT_IOT_USERNAME", "iotuser"),
        ("DEFAULT_IOT_PASSWORD", "iotpass"),
        ("DEFAULT_IOT_IP", "192.168.1.2:22"),
    ];

    for (var, default_value) in vars {
        let value = env::var(var).unwrap_or_else(|_| default_value.to_string());
        println!("cargo:rustc-env={}={}", var, value);
    }

    // Tell Cargo to re-run this script if .env changes, but don't fail if it doesn't exist
    println!("cargo:rerun-if-changed=.env");
}
