use roxmltree::Document;

#[derive(Clone)]
pub struct NamespaceInfo {
    number: String,
    file_name: String,
}

#[derive(Clone)]
pub struct OpcuaConfig<'a> {
    // Connection settings
    pub ip: &'a str,
    pub username: &'a str,
    pub password: &'a str,
    pub is_listener: bool,

    // Group settings
    pub group_name: &'a str,
    pub namespace_number: &'a str,
    pub interval_ms: u64, // Store as u64 and format when needed
}

impl OpcuaConfig<'_> {
    fn get_interval_string(&self) -> String {
        format!("{}ms", self.interval_ms)
    }
}

pub fn format_config_header(
    influx_token: &str,
    bucket_name: &str,
    config_strings: &[String],
    namespace_infos: &[NamespaceInfo],
) -> String {
    let namespace_comments = namespace_infos
        .iter()
        .map(|ns| format!("# Namespace for file {}: {}", ns.file_name, ns.number))
        .collect::<Vec<String>>()
        .join("\n");

    format!(
        r#"{}

##################################

# Global tags can be specified here in key="value" format.
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
  bucket = "{}"

{}
"#,
        namespace_comments,
        influx_token,
        bucket_name,
        config_strings.join("\n\n")
    )
}

fn format_config(config: &OpcuaConfig, nodes_str: &str) -> String {
    let input_type = if config.is_listener {
        "opcua_listener"
    } else {
        "opcua"
    };
    let name = if config.is_listener {
        "opcua_listener"
    } else {
        "opcua"
    }; // #TODO: find out if this is necessary
    let session_timeout = if config.is_listener { "20m" } else { "5m" };
    let interval_key = if config.is_listener {
        "sampling_interval"
    } else {
        "interval"
    };
    let extra_config = if config.is_listener {
        "connect_fail_behavior = \"ignore\"\n  "
    } else {
        ""
    };
    let interval = config.get_interval_string();

    format!(
        r#"
[[inputs.{input_type}]]
  name = "{name}"
  endpoint = "opc.tcp://{}:4840"
  {extra_config}connect_timeout = "300s"
  request_timeout = "10s"
  session_timeout = "{session_timeout}"
  security_policy = "Basic256Sha256"
  security_mode = "SignAndEncrypt"
  certificate = ""
  private_key = ""
  auth_method = "UserName"
  username = "{}"
  password = "{}"
  timestamp = "source"
  client_trace = false
    [[inputs.{input_type}.group]]
      name = "{}"
      {interval_key} = "{}"
      namespace = "{}"
      identifier_type = "i"
      nodes = [
        {}
      ]
    "#,
        config.ip,
        config.username,
        config.password,
        config.group_name,
        interval,
        config.namespace_number,
        nodes_str
    )
}

pub fn parse_xml(
    config: &OpcuaConfig,
    xml_file: &str,
    namespace_infos: &mut Vec<NamespaceInfo>,
) -> String {
    let xml = std::fs::read_to_string(xml_file).expect("Unable to read file");
    let doc = Document::parse(&xml).expect("Unable to parse XML");

    let file_name = std::path::Path::new(xml_file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    namespace_infos.push(NamespaceInfo {
        number: config.namespace_number.to_string(),
        file_name,
    });

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
        display_name
    } else {
        //TODO: should always find one tbh
        std::path::Path::new(xml_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    };

    let config_with_group = OpcuaConfig {
        group_name: &group_name,
        ..config.clone()
    };

    format_config(&config_with_group, &nodes_str)
}
