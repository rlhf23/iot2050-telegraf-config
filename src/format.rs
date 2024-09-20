use roxmltree::Document;

pub fn generate_config_content(influx_token: &str, config_strings: &[String]) -> String {
    format!(
        r#"# Global tags can be specified here in key="value" format.
[global_tags]

# Configuration for telegraf agent
[agent]
  ## Default data collection interval for all inputs
  interval = "1000ms"
  round_interval = true

  metric_batch_size = 10000
  metric_buffer_limit = 100000

  collection_jitter = "0s"
  flush_interval = "10s"
  flush_jitter = "0s"
  precision = "0s"

  ## Log at debug level.
  # debug = false
  ## Log only error level messages.
  # quiet = false

  logtarget = "file"
  logfile = "/var/log/telegraf/telegraf.log"
  logfile_rotation_max_size = "25MB"
  logfile_rotation_max_archives = 4

  hostname = ""
  omit_hostname = false

# Configuration for sending metrics to InfluxDB 2.0
[[outputs.influxdb_v2]]
  urls = ["http://127.0.0.1:8086"]
  token = "{}"
  organization = "org"
  bucket = "line"

{}
"#,
        influx_token,
        config_strings.join("\n\n")
    )
}

fn format_standard_config(
    ip: &str,
    username: &str,
    password: &str,
    group_name: &str,
    namespace_number: &str,
    interval: &str,
    nodes_str: &str,
) -> String {
    format!(
        r#"
[[inputs.opcua]]
name = "opcua"
interval = "{}"
endpoint = "opc.tcp://{}:4840"
connect_timeout = "30s"
request_timeout = "10s"
security_policy = "Basic256Sha256"
security_mode = "SignAndEncrypt"
certificate = ""
private_key = ""
auth_method = "UserName"
username = "{}"
password = "{}"
timestamp = "source"
client_trace = false
    [[inputs.opcua.group]]
      name = "{}"
      namespace = "{}"
      identifier_type = "i"
      nodes = [
        {}
      ]
    "#,
        interval, ip, username, password, group_name, namespace_number, nodes_str
    )
}

fn format_listener_config(
    ip: &str,
    username: &str,
    password: &str,
    group_name: &str,
    namespace_number: &str,
    interval: &str,
    nodes_str: &str,
) -> String {
    format!(
        r#"
[[inputs.opcua_listener]]
name = "opcua_listener"
endpoint = "opc.tcp://{}:4840"
connect_fail_behavior = "ignore"
connect_timeout = "30s"
request_timeout = "10s"
session_timeout = "20m"
security_policy = "Basic256Sha256"
security_mode = "SignAndEncrypt"
certificate = ""
private_key = ""
auth_method = "UserName"
username = "{}"
password = "{}"
timestamp = "source"
client_trace = false
    [[inputs.opcua_listener.group]]
      name = "{}"
      sampling_interval = "{}"
      namespace = "{}"
      identifier_type = "i"
      nodes = [
        {}
      ]
    "#,
        ip, username, password, group_name, interval, namespace_number, nodes_str
    )
}

pub fn parse_xml(
    xml_file: &str,
    ip: &str,
    username: &str,
    password: &str,
    is_listener: bool,
) -> String {
    let xml = std::fs::read_to_string(xml_file).expect("Unable to read file");
    let doc = Document::parse(&xml).expect("Unable to parse XML");

    // asking for individual namespace numbers
    println!("----Enter the namespace number for {}:", xml_file);
    let mut namespace_number = String::new();
    std::io::stdin().read_line(&mut namespace_number).unwrap();
    let namespace_number = namespace_number.trim();

    // ask for intervals
    let mut interval = String::new();
    let interval_input = if !is_listener {
        println!("{}", "----Enter the interval in ms (default 1000ms):");
        std::io::stdin().read_line(&mut interval).unwrap();
        interval.trim()
    } else {
        println!(
            "{}",
            "----Enter the sampling_interval in ms (default 1000ms):"
        );
        std::io::stdin().read_line(&mut interval).unwrap();
        interval.trim()
    };

    let interval = if interval_input.is_empty() {
        if !is_listener {
            "1000ms"
        } else {
            "1000ms"
        }
    } else {
        interval_input
    };

    let mut nodes = Vec::new();

    let mut display_name = String::new();
    for variable in doc.descendants().filter(|n| n.has_tag_name("UAObject")) {
        let node_id = variable.attribute("NodeId");
        // Check for the specific node and print its DisplayName
        if let Some(node_id) = node_id {
            if node_id == "ns=2;i=1" {
                if let Some(found_name) = variable
                    .descendants()
                    .find(|n| n.has_tag_name("DisplayName"))
                    .and_then(|n| n.text())
                {
                    display_name = found_name.to_string();
                    println!("##BrowseName for ns=2;i=1: {}", found_name);
                }
            }
        }
    }
    for variable in doc.descendants().filter(|n| n.has_tag_name("UAVariable")) {
        let node_id = variable.attribute("NodeId");
        if let Some(node_id) = node_id {
            if node_id.starts_with("ns=2;i=") {
                let identifier = node_id.split('=').nth(2).unwrap().to_string();

                let mut name = variable
                    .descendants()
                    .find(|n| n.has_tag_name("BrowseName"))
                    .and_then(|n| n.text())
                    .unwrap_or_default()
                    .to_string();

                if let Some(var_mapping) = variable
                    .descendants()
                    .find(|n| n.has_tag_name("VariableMapping"))
                    .and_then(|n| n.text())
                {
                    let var_mapping = var_mapping.replace('"', "");
                    name = var_mapping;
                }

                nodes.push(format!(
                    "{{name=\"{}\", identifier=\"{}\"}}",
                    name, identifier
                ));
            }
        }
    }

    let nodes_str = nodes.join(",\n        ");

    let group_name = if !display_name.is_empty() {
        display_name.to_string()
    } else {
        std::path::Path::new(xml_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    };

    if is_listener {
        format_listener_config(
            ip,
            username,
            password,
            &group_name,
            namespace_number,
            interval,
            &nodes_str,
        )
    } else {
        format_standard_config(
            ip,
            username,
            password,
            &group_name,
            namespace_number,
            interval,
            &nodes_str,
        )
    }
}
