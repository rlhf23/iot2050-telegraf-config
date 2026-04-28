use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};
use uuid::Uuid;

// Import from main project
use sie_generate_config::{TelegrafConfig, backend::{ConfigGenerator, OutputFormat}};

const UPLOAD_DIR: &str = "/tmp/config-uploads";
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Ensure OPC-UA IP has port appended (default 4840)
fn ensure_opcua_port(ip: String) -> String {
    if ip.contains(':') {
        ip
    } else {
        format!("{}:4840", ip)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub success: bool,
    pub session_id: String,
}

#[derive(Serialize)]
pub struct UploadedFile {
    pub name: String,
    pub size: u64,
    pub path: String,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub message: String,
    pub uploaded_files: Vec<UploadedFile>,
}

#[derive(Serialize)]
pub struct FileListResponse {
    pub success: bool,
    pub files: Vec<UploadedFile>,
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct FileConfig {
    pub filename: String,
    pub namespace: String,
    pub interval_ms: u64,
    pub use_listener: bool,
    pub custom_ip: Option<String>, // Per-file custom OPC-UA server IP
}

#[derive(Deserialize)]
pub struct GenerateConfigRequest {
    pub session_id: String,
    pub opcua_ip: Option<String>,
    pub opcua_username: Option<String>,
    pub opcua_password: Option<String>,
    pub anonymous: bool,
    pub iot_host: Option<String>,
    pub iot_username: Option<String>,
    pub iot_password: Option<String>,
    pub output_format: String,
    pub file_configs: Vec<FileConfig>,
    #[serde(default)]
    pub use_source_timestamp: bool, // Use "source" instead of "gather" for timestamp
    #[serde(default)]
    pub include_test_inputs: bool, // Include system metrics (CPU, memory, disk)
    #[serde(default)]
    pub include_opcua_diagnostics: bool, // Include OPC UA server diagnostics
}

#[derive(Serialize)]
pub struct GenerateConfigResponse {
    pub success: bool,
    pub message: String,
    pub config_path: Option<String>,
    pub preview: Option<String>,
    pub measurements: Option<Vec<String>>,
}

/// Create a new session
pub async fn create_session() -> Json<SessionResponse> {
    let session_id = Uuid::new_v4().to_string();
    
    info!("Created new session: {}", session_id);
    
    Json(SessionResponse {
        success: true,
        session_id,
    })
}

/// Get the session directory path
fn get_session_dir(session_id: &str) -> PathBuf {
    PathBuf::from(UPLOAD_DIR).join(session_id).join("uploads")
}

/// Validate that a filename is safe (no path traversal)
fn is_safe_filename(filename: &str) -> bool {
    !filename.contains("..") && !filename.contains('/') && !filename.contains('\\')
}

/// Validate that a file is XML
fn is_xml_file(filename: &str) -> bool {
    filename.to_lowercase().ends_with(".xml")
}

/// Upload XML configuration files
pub async fn upload_files(
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, Json<UploadResponse>)> {
    let mut session_id: Option<String> = None;
    let mut uploaded_files = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                error!("Failed to read multipart field: {}", e);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(UploadResponse {
                        success: false,
                        message: "Failed to read upload data".to_string(),
                        uploaded_files: vec![],
                    }),
                ));
            }
        };
        let name = field.name().unwrap_or("").to_string();

        if name == "session_id" {
            // Extract session ID
            let data = field.bytes().await.unwrap_or_default();
            session_id = Some(String::from_utf8_lossy(&data).to_string());
        } else if name == "files" {
            // Extract file
            let filename = field.file_name().unwrap_or("unknown").to_string();
            
            // Validate filename
            if !is_safe_filename(&filename) {
                warn!("Rejected unsafe filename: {}", filename);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(UploadResponse {
                        success: false,
                        message: format!("Invalid filename: {}", filename),
                        uploaded_files: vec![],
                    }),
                ));
            }

            // Validate file type
            if !is_xml_file(&filename) {
                warn!("Rejected non-XML file: {}", filename);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(UploadResponse {
                        success: false,
                        message: format!("Only XML files are allowed: {}", filename),
                        uploaded_files: vec![],
                    }),
                ));
            }

            // Get file data
            let data = match field.bytes().await {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read file data: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(UploadResponse {
                            success: false,
                            message: "Failed to read file data".to_string(),
                            uploaded_files: vec![],
                        }),
                    ));
                }
            };

            // Check file size
            if data.len() > MAX_FILE_SIZE {
                warn!("File too large: {} ({} bytes)", filename, data.len());
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(UploadResponse {
                        success: false,
                        message: format!("File too large: {} (max 10MB)", filename),
                        uploaded_files: vec![],
                    }),
                ));
            }

            // Validate XML structure (basic check)
            let content = String::from_utf8_lossy(&data);
            if !content.trim_start().starts_with("<?xml") && !content.trim_start().starts_with("<") {
                warn!("Invalid XML file: {}", filename);
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(UploadResponse {
                        success: false,
                        message: format!("Invalid XML file: {}", filename),
                        uploaded_files: vec![],
                    }),
                ));
            }

            // Store session_id and file info for later processing
            if let Some(ref sid) = session_id {
                let session_dir = get_session_dir(sid);
                
                // Create session directory if it doesn't exist
                if let Err(e) = fs::create_dir_all(&session_dir).await {
                    error!("Failed to create session directory: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(UploadResponse {
                            success: false,
                            message: "Failed to create upload directory".to_string(),
                            uploaded_files: vec![],
                        }),
                    ));
                }

                // Write file
                let file_path = session_dir.join(&filename);
                match fs::File::create(&file_path).await {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(&data).await {
                            error!("Failed to write file: {}", e);
                            return Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(UploadResponse {
                                    success: false,
                                    message: "Failed to save file".to_string(),
                                    uploaded_files: vec![],
                                }),
                            ));
                        }

                        info!("Uploaded file: {} ({} bytes)", filename, data.len());

                        uploaded_files.push(UploadedFile {
                            name: filename.clone(),
                            size: data.len() as u64,
                            path: file_path.to_string_lossy().to_string(),
                        });
                    }
                    Err(e) => {
                        error!("Failed to create file: {}", e);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(UploadResponse {
                                success: false,
                                message: "Failed to create file".to_string(),
                                uploaded_files: vec![],
                            }),
                        ));
                    }
                }
            }
        }
    }

    if session_id.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(UploadResponse {
                success: false,
                message: "Missing session_id".to_string(),
                uploaded_files: vec![],
            }),
        ));
    }

    if uploaded_files.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(UploadResponse {
                success: false,
                message: "No files uploaded".to_string(),
                uploaded_files: vec![],
            }),
        ));
    }

    Ok(Json(UploadResponse {
        success: true,
        message: format!("Successfully uploaded {} file(s)", uploaded_files.len()),
        uploaded_files,
    }))
}

/// List uploaded files for a session
pub async fn list_files(
    Path(session_id): Path<String>,
) -> Result<Json<FileListResponse>, (StatusCode, Json<FileListResponse>)> {
    let session_dir = get_session_dir(&session_id);

    // Check if session directory exists
    if !session_dir.exists() {
        return Ok(Json(FileListResponse {
            success: true,
            files: vec![],
        }));
    }

    // Read directory
    let mut files = Vec::new();
    match fs::read_dir(&session_dir).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_file() {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        if is_xml_file(&filename) {
                            files.push(UploadedFile {
                                name: filename.clone(),
                                size: metadata.len(),
                                path: entry.path().to_string_lossy().to_string(),
                            });
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to read session directory: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FileListResponse {
                    success: false,
                    files: vec![],
                }),
            ));
        }
    }

    Ok(Json(FileListResponse {
        success: true,
        files,
    }))
}

/// Delete a file from a session
pub async fn delete_file(
    Path((session_id, filename)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<DeleteResponse>)> {
    // Validate filename
    if !is_safe_filename(&filename) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(DeleteResponse {
                success: false,
                message: "Invalid filename".to_string(),
            }),
        ));
    }

    let session_dir = get_session_dir(&session_id);
    let file_path = session_dir.join(&filename);

    // Check if file exists
    if !file_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(DeleteResponse {
                success: false,
                message: "File not found".to_string(),
            }),
        ));
    }

    // Delete file
    match fs::remove_file(&file_path).await {
        Ok(_) => {
            info!("Deleted file: {}", filename);
            Ok(Json(DeleteResponse {
                success: true,
                message: format!("File '{}' deleted successfully", filename),
            }))
        }
        Err(e) => {
            error!("Failed to delete file: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DeleteResponse {
                    success: false,
                    message: "Failed to delete file".to_string(),
                }),
            ))
        }
    }
}

/// Clean up old sessions (files older than 24 hours)
pub async fn cleanup_old_sessions() -> Result<(), std::io::Error> {
    let upload_dir = PathBuf::from(UPLOAD_DIR);
    
    if !upload_dir.exists() {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let mut entries = fs::read_dir(&upload_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        if let Ok(metadata) = entry.metadata().await {
            if metadata.is_dir() {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                    let age = now.signed_duration_since(modified_time);
                    
                    // Delete sessions older than 24 hours
                    if age.num_hours() > 24 {
                        let session_path = entry.path();
                        info!("Cleaning up old session: {:?}", session_path);
                        let _ = fs::remove_dir_all(&session_path).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Generate Telegraf configuration from uploaded files
pub async fn generate_config(
    Json(request): Json<GenerateConfigRequest>,
) -> Result<Json<GenerateConfigResponse>, (StatusCode, Json<GenerateConfigResponse>)> {
    info!("Generating config for session: {}", request.session_id);

    let session_dir = get_session_dir(&request.session_id);
    
    // Check if session directory exists
    if !session_dir.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(GenerateConfigResponse {
                success: false,
                message: "Session not found".to_string(),
                config_path: None,
                preview: None,
                measurements: None,
            }),
        ));
    }

    // Validate file configs
    if request.file_configs.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenerateConfigResponse {
                success: false,
                message: "No file configurations provided".to_string(),
                config_path: None,
                preview: None,
                measurements: None,
            }),
        ));
    }

    // Verify all files exist
    for file_config in &request.file_configs {
        let file_path = session_dir.join(&file_config.filename);
        if !file_path.exists() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(GenerateConfigResponse {
                    success: false,
                    message: format!("File not found: {}", file_config.filename),
                    config_path: None,
                    preview: None,
                    measurements: None,
                }),
            ));
        }
    }

    // Determine OPC-UA credentials
    let (username, password) = if request.anonymous {
        ("".to_string(), "".to_string())
    } else {
        (
            request.opcua_username.unwrap_or_default(),
            request.opcua_password.unwrap_or_default(),
        )
    };

    // Create TelegrafConfig for ConfigGenerator
    // Use sensible defaults for web UI (local deployment, no SSH needed)
    let telegraf_config = TelegrafConfig {
        folder: session_dir.clone(),
        ip: ensure_opcua_port(
            request.opcua_ip.clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "127.0.0.1".to_string())
        ),
        username: username.clone(),
        password: password.clone(),
        iot_host: request.iot_host.clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "127.0.0.1:22".to_string()),
        iot_username: request.iot_username.clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "user".to_string()),
        iot_password: request.iot_password.clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "pass".to_string()),
        listener_files: request.file_configs.iter()
            .filter(|fc| fc.use_listener)
            .map(|fc| fc.filename.clone())
            .collect(),
        output_format: Some(request.output_format.clone()),
        include_test_inputs: request.include_test_inputs,
        include_opcua_diagnostics: request.include_opcua_diagnostics,
        selected_opcua_nodes: vec![],
        use_source_timestamp: request.use_source_timestamp,
        ship_display_name: None,
        ship_hostname: None,
    };

    // Create ConfigGenerator
    let mut generator = match ConfigGenerator::new(telegraf_config) {
        Ok(gen) => gen,
        Err(e) => {
            error!("Failed to create ConfigGenerator: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenerateConfigResponse {
                    success: false,
                    message: format!("Failed to create ConfigGenerator: {}", e),
                    config_path: None,
                    preview: None,
                    measurements: None,
                }),
            ));
        }
    };

    // Set file configurations
    for fc in &request.file_configs {
        // Use per-file custom IP if provided, otherwise use main OPC-UA IP, fallback to 127.0.0.1
        let file_ip = ensure_opcua_port(
            fc.custom_ip.clone()
                .filter(|s| !s.is_empty())
                .or_else(|| request.opcua_ip.clone().filter(|s| !s.is_empty()))
                .unwrap_or_else(|| "127.0.0.1".to_string())
        );
        
        // Build full path to the file
        let file_path = session_dir.join(&fc.filename).to_string_lossy().to_string();
        
        generator.set_file_config(
            file_path,
            fc.namespace.clone(),
            fc.interval_ms,
            Some(file_ip),
        );
    }

    // Set output format
    let output_format = match request.output_format.as_str() {
        "prometheus" => OutputFormat::Prometheus,
        _ => OutputFormat::InfluxDB,
    };
    generator.set_output_format(output_format);

    // Create generated directory
    let generated_dir = PathBuf::from(UPLOAD_DIR)
        .join(&request.session_id)
        .join("generated");
    
    if let Err(e) = fs::create_dir_all(&generated_dir).await {
        error!("Failed to create generated directory: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenerateConfigResponse {
                success: false,
                message: "Failed to create output directory".to_string(),
                config_path: None,
                preview: None,
                measurements: None,
            }),
        ));
    }

    // Collect XML file paths (need full paths, not just filenames)
    let xml_files: Vec<String> = request.file_configs.iter()
        .map(|fc| session_dir.join(&fc.filename).to_string_lossy().to_string())
        .collect();

    // Collect listener file paths
    let listener_files: Vec<String> = request.file_configs.iter()
        .filter(|fc| fc.use_listener)
        .map(|fc| session_dir.join(&fc.filename).to_string_lossy().to_string())
        .collect();

    // Generate configuration
    let output_path = generated_dir.join("telegraf.conf");
    match generator.generate_config(&xml_files, &listener_files) {
        Ok(result) => {
            // Write config to file
            if let Err(e) = fs::write(&output_path, &result.config_content).await {
                error!("Failed to write config file: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenerateConfigResponse {
                        success: false,
                        message: format!("Failed to write configuration file: {}", e),
                        config_path: None,
                        preview: None,
                        measurements: None,
                    }),
                ));
}
 
            info!("Successfully generated config for session: {}", request.session_id);

            // Write OPC UA credentials for the PLC time endpoint
            let file_configs_json: Vec<serde_json::Value> = request.file_configs.iter().map(|fc| {
                serde_json::json!({
                    "filename": fc.filename,
                    "namespace": fc.namespace,
                    "interval_ms": fc.interval_ms,
                    "use_listener": fc.use_listener,
                    "custom_ip": fc.custom_ip,
                })
            }).collect();
            let credentials = serde_json::json!({
                "endpoint": ensure_opcua_port(request.opcua_ip.clone().filter(|s| !s.is_empty()).unwrap_or_default()),
                "username": username,
                "password": password,
                "anonymous": request.anonymous,
                "file_configs": file_configs_json,
            });
            let creds_path = generated_dir.join("opcua-credentials.json");
            if let Err(e) = fs::write(&creds_path, serde_json::to_string_pretty(&credentials).unwrap_or_default()).await {
                info!("Note: Failed to write OPC UA credentials file: {}", e);
            }

            // Return full config content without truncation
            let preview = result.config_content;

            Ok(Json(GenerateConfigResponse {
                success: true,
                message: format!("Configuration generated successfully from {} file(s)", request.file_configs.len()),
                config_path: Some(output_path.to_string_lossy().to_string()),
                preview: Some(preview),
                measurements: Some(result.measurements),
            }))
        }
        Err(e) => {
            error!("Failed to generate config: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenerateConfigResponse {
                    success: false,
                    message: format!("Failed to generate configuration: {}", e),
                    config_path: None,
                    preview: None,
                    measurements: None,
                }),
            ))
        }
    }
}
