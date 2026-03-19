//! Grafana dashboard generation from templates
//!
//! This module provides functionality to generate Grafana dashboards
//! from JSON templates with placeholder substitution.

use crate::error::TelegrafError;
use std::fs;
use std::path::Path;

/// Default embedded template for dashboards
const DEFAULT_TEMPLATE: &str = include_str!("../../config/grafana/dashboard_template.json");

/// Configuration for dashboard generation
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Unique identifier for the dashboard (used in URL)
    pub uid: String,
    /// Display title for the dashboard
    pub title: String,
    /// InfluxDB measurement name(s) to query
    pub measurements: Vec<String>,
    /// InfluxDB bucket name
    pub bucket: String,
    /// Grafana datasource UID (or name for name-based reference)
    pub datasource_uid: String,
}

/// Load dashboard template from file or use default embedded template
pub fn load_template(template_path: Option<&Path>) -> Result<String, TelegrafError> {
    match template_path {
        Some(path) => fs::read_to_string(path).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to load dashboard template: {}", e))
        }),
        None => Ok(DEFAULT_TEMPLATE.to_string()),
    }
}

/// Sanitize a string for use as a dashboard UID
/// Replaces invalid characters with underscores and converts to lowercase
pub fn sanitize_uid(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

/// Replace placeholders in template with actual values
pub fn fill_template(template: &str, config: &DashboardConfig) -> Result<String, TelegrafError> {
    let mut result = template.to_string();

    // Replace simple placeholders
    result = result.replace("{{DASHBOARD_UID}}", &config.uid);
    result = result.replace("{{DASHBOARD_TITLE}}", &config.title);
    result = result.replace("{{DATASOURCE_UID}}", &config.datasource_uid);
    result = result.replace("{{BUCKET}}", &config.bucket);

    // For measurements, use the first measurement for single-panel template
    // (Multi-panel templates would need more complex handling)
    if let Some(measurement) = config.measurements.first() {
        result = result.replace("{{MEASUREMENT}}", measurement);
    } else {
        return Err(TelegrafError::ConfigError(
            "At least one measurement is required for dashboard generation".to_string(),
        ));
    }

    Ok(result)
}

/// Generate a dashboard JSON from configuration
///
/// # Arguments
/// * `config` - Dashboard configuration
/// * `template_path` - Optional path to custom template file
///
/// # Returns
/// * `Ok(String)` - Generated dashboard JSON
/// * `Err(TelegrafError)` - Template loading or substitution error
pub fn generate_dashboard(
    config: &DashboardConfig,
    template_path: Option<&Path>,
) -> Result<String, TelegrafError> {
    let template = load_template(template_path)?;
    fill_template(&template, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_uid_simple() {
        assert_eq!(sanitize_uid("Sample_DB"), "sample_db");
        assert_eq!(sanitize_uid("MyMeasurement"), "mymeasurement");
    }

    #[test]
    fn test_sanitize_uid_special_chars() {
        assert_eq!(sanitize_uid("Sample DB"), "sample_db");
        assert_eq!(sanitize_uid("My/Measurement"), "my_measurement");
        assert_eq!(sanitize_uid("Test.Metric"), "test_metric");
    }

    #[test]
    fn test_sanitize_uid_preserves_valid_chars() {
        assert_eq!(sanitize_uid("my-measurement"), "my-measurement");
        assert_eq!(sanitize_uid("my_measurement"), "my_measurement");
    }

    #[test]
    fn test_load_default_template() {
        let template = load_template(None).unwrap();
        assert!(template.contains("{{DASHBOARD_UID}}"));
        assert!(template.contains("{{MEASUREMENT}}"));
        assert!(template.contains("{{BUCKET}}"));
    }

    #[test]
    fn test_fill_template_basic() {
        let config = DashboardConfig {
            uid: "sample_db".to_string(),
            title: "Sample_DB Monitoring".to_string(),
            measurements: vec!["Sample_DB".to_string()],
            bucket: "telegraf".to_string(),
            datasource_uid: "InfluxDB".to_string(),
        };

        let template = r#"{"dashboard":{"uid":"{{DASHBOARD_UID}}","title":"{{DASHBOARD_TITLE}}","panels":[{"title":"{{MEASUREMENT}}","targets":[{"query":"from(bucket: {{BUCKET}})"}]}]},"overwrite":true}"#;

        let result = fill_template(template, &config).unwrap();

        assert!(result.contains("\"uid\":\"sample_db\""));
        assert!(result.contains("\"title\":\"Sample_DB Monitoring\""));
        assert!(result.contains("Sample_DB"));
        assert!(result.contains("telegraf"));
    }

    #[test]
    fn test_fill_template_all_placeholders() {
        let config = DashboardConfig {
            uid: "test_uid".to_string(),
            title: "Test Dashboard".to_string(),
            measurements: vec!["TestMeasurement".to_string()],
            bucket: "my_bucket".to_string(),
            datasource_uid: "MyDataSource".to_string(),
        };

        let template = load_template(None).unwrap();
        let result = fill_template(&template, &config).unwrap();

        // Verify all placeholders are replaced
        assert!(!result.contains("{{DASHBOARD_UID}}"));
        assert!(!result.contains("{{DASHBOARD_TITLE}}"));
        assert!(!result.contains("{{MEASUREMENT}}"));
        assert!(!result.contains("{{BUCKET}}"));
        assert!(!result.contains("{{DATASOURCE_UID}}"));

        // Verify values are present
        assert!(result.contains("test_uid"));
        assert!(result.contains("Test Dashboard"));
        assert!(result.contains("TestMeasurement"));
        assert!(result.contains("my_bucket"));
        assert!(result.contains("MyDataSource"));
    }

    #[test]
    fn test_fill_template_empty_measurements() {
        let config = DashboardConfig {
            uid: "test".to_string(),
            title: "Test".to_string(),
            measurements: vec![],
            bucket: "telegraf".to_string(),
            datasource_uid: "InfluxDB".to_string(),
        };

        let template = "{{MEASUREMENT}}";
        let result = fill_template(template, &config);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one measurement"));
    }

    #[test]
    fn test_generate_dashboard_with_default_template() {
        let config = DashboardConfig {
            uid: "opcua_sample".to_string(),
            title: "OPC UA Sample Data".to_string(),
            measurements: vec!["Sample_DB".to_string()],
            bucket: "telegraf".to_string(),
            datasource_uid: "InfluxDB".to_string(),
        };

        let result = generate_dashboard(&config, None).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["dashboard"]["uid"], "opcua_sample");
        assert_eq!(parsed["dashboard"]["title"], "OPC UA Sample Data");
        assert_eq!(parsed["overwrite"], true);
    }

    #[test]
    fn test_generate_dashboard_valid_json() {
        let config = DashboardConfig {
            uid: "test".to_string(),
            title: "Test".to_string(),
            measurements: vec!["mymeasurement".to_string()],
            bucket: "telegraf".to_string(),
            datasource_uid: "InfluxDB".to_string(),
        };

        let result = generate_dashboard(&config, None).unwrap();

        // Should parse as valid JSON
        let _: serde_json::Value = serde_json::from_str(&result).unwrap();
    }
}
