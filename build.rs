use dotenv::dotenv;
use std::env;

fn main() {
    // Load .env file
    dotenv().ok();

    // List of environment variables to process with default values
    let vars = vec![
        ("DEFAULT_IP", "192.168.1.1"),
        ("DEFAULT_USERNAME", "admin"),
        ("DEFAULT_PASSWORD", "password"),
        ("DEFAULT_IOT_USERNAME", "root"),
        ("DEFAULT_IOT_PASSWORD", "iotpassword"),
        ("DEFAULT_IOT_IP", "192.168.1.100:22"),
    ];

    for (var, default_value) in vars {
        let value = env::var(var).unwrap_or_else(|_| default_value.to_string());
        println!("cargo:rustc-env={}={}", var, value);
    }

    // Tell Cargo to re-run this script if .env changes
    println!("cargo:rerun-if-changed=.env");
}
