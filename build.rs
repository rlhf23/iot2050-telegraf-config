use dotenv::dotenv;
use std::env;

fn main() {
    // Load .env file
    dotenv().ok();

    // List of environment variables to process
    let vars = vec![
        "DEFAULT_IP",
        "DEFAULT_USERNAME",
        "DEFAULT_PASSWORD",
        "DEFAULT_IOT_IP",
        "DEFAULT_IOT_PASSWORD",
    ];

    for var in vars {
        if let Ok(value) = env::var(var) {
            println!("cargo:rustc-env={}={}", var, value);
        } else {
            panic!("Environment variable {} not set", var);
        }
    }

    // Tell Cargo to re-run this script if .env changes
    println!("cargo:rerun-if-changed=.env");
}
