//! Chronograf dashboard generation from templates
//!
//! This module provides functionality to generate Chronograf dashboards
//! from JSON templates with placeholder substitution, mirroring the
//! Grafana dashboard generation pattern.

use crate::error::TelegrafError;
use std::fs;
use std::path::Path;

const DEFAULT_TEMPLATE: &str =
    include_str!("../../docker/config/chronograf/templates/dashboard_template.json");

const CELLS_PER_ROW: i64 = 3;
const CELL_WIDTH: i64 = 4;
const CELL_HEIGHT: i64 = 4;
const DASHBOARD_WIDTH: i64 = 12;

#[derive(Debug, Clone)]
pub struct ChronografDashboardConfig {
    pub name: String,
    pub organization: String,
    pub measurements: Vec<String>,
    pub bucket: String,
}

pub fn load_template(template_path: Option<&Path>) -> Result<String, TelegrafError> {
    match template_path {
        Some(path) => fs::read_to_string(path).map_err(|e| {
            TelegrafError::ConfigError(format!(
                "Failed to load Chronograf dashboard template: {}",
                e
            ))
        }),
        None => Ok(DEFAULT_TEMPLATE.to_string()),
    }
}

fn generate_cell_id(index: usize) -> String {
    format!("{:08x}-0001-4000-8000-{:012x}", index, index)
}

fn generate_cell(
    cell_template: &serde_json::Value,
    measurement: &str,
    bucket: &str,
    index: usize,
) -> Result<serde_json::Value, TelegrafError> {
    let mut cell = cell_template.clone();

    let cell_str = serde_json::to_string(&cell)
        .map_err(|e| TelegrafError::ConfigError(format!("Failed to serialize cell: {}", e)))?;

    let cell_id = generate_cell_id(index);
    let cell_str = cell_str
        .replace("{{CELL_ID}}", &cell_id)
        .replace("{{MEASUREMENT}}", measurement)
        .replace("{{BUCKET}}", bucket);

    cell = serde_json::from_str(&cell_str)
        .map_err(|e| TelegrafError::ConfigError(format!("Failed to parse cell JSON: {}", e)))?;

    // Position cells in a grid layout: 3 columns, CELL_HEIGHT rows
    let row = (index as i64) / CELLS_PER_ROW;
    let col = (index as i64) % CELLS_PER_ROW;

    if let Some(obj) = cell.as_object_mut() {
        if let Some(x) = obj.get_mut("x") {
            *x = serde_json::json!(col * CELL_WIDTH);
        }
        if let Some(y) = obj.get_mut("y") {
            *y = serde_json::json!(row * CELL_HEIGHT);
        }
    }

    Ok(cell)
}

pub fn fill_template(
    template: &str,
    config: &ChronografDashboardConfig,
) -> Result<String, TelegrafError> {
    if config.measurements.is_empty() {
        return Err(TelegrafError::ConfigError(
            "At least one measurement is required for Chronograf dashboard generation".to_string(),
        ));
    }

    let mut template_json: serde_json::Value = serde_json::from_str(template).map_err(|e| {
        TelegrafError::ConfigError(format!("Invalid Chronograf template JSON: {}", e))
    })?;

    let cell_template = template_json
        .get("__cell_template")
        .cloned()
        .ok_or_else(|| {
            TelegrafError::ConfigError(
                "Chronograf template missing __cell_template field".to_string(),
            )
        })?;

    if let Some(obj) = template_json.as_object_mut() {
        obj.remove("__cell_template");
    }

    let meta = template_json.get_mut("meta").ok_or_else(|| {
        TelegrafError::ConfigError("Chronograf template missing meta field".to_string())
    })?;

    if let Some(meta_obj) = meta.as_object_mut() {
        meta_obj.insert("chronografVersion".to_string(), serde_json::json!("1.10.9"));
        meta_obj.insert("sources".to_string(), serde_json::json!({}));
    }

    let dashboard = template_json.get_mut("dashboard").ok_or_else(|| {
        TelegrafError::ConfigError("Chronograf template missing dashboard field".to_string())
    })?;

    let dashboard_obj = dashboard.as_object_mut().ok_or_else(|| {
        TelegrafError::ConfigError("Chronograf dashboard field is not an object".to_string())
    })?;

    dashboard_obj.insert("name".to_string(), serde_json::json!(config.name));
    dashboard_obj.insert(
        "organization".to_string(),
        serde_json::json!(config.organization),
    );

    let mut cells = Vec::new();
    for (index, measurement) in config.measurements.iter().enumerate() {
        let cell = generate_cell(&cell_template, measurement, &config.bucket, index)?;
        cells.push(cell);
    }

    dashboard_obj.insert("cells".to_string(), serde_json::json!(cells));
    dashboard_obj.insert("templates".to_string(), serde_json::json!([]));

    serde_json::to_string_pretty(&template_json).map_err(|e| {
        TelegrafError::ConfigError(format!("Failed to serialize Chronograf dashboard: {}", e))
    })
}

pub fn generate_chronograf_dashboard(
    config: &ChronografDashboardConfig,
    template_path: Option<&Path>,
) -> Result<String, TelegrafError> {
    let template = load_template(template_path)?;
    fill_template(&template, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_template() {
        let template = load_template(None).unwrap();
        assert!(template.contains("{{DASHBOARD_NAME}}"));
        assert!(template.contains("__cell_template"));
    }

    #[test]
    fn test_fill_template_single_measurement() {
        let config = ChronografDashboardConfig {
            name: "Test Dashboard".to_string(),
            organization: "default".to_string(),
            measurements: vec!["cpu".to_string()],
            bucket: "telegraf".to_string(),
        };

        let template = load_template(None).unwrap();
        let result = fill_template(&template, &config).unwrap();

        assert!(!result.contains("{{DASHBOARD_NAME}}"));
        assert!(!result.contains("{{MEASUREMENT}}"));
        assert!(!result.contains("{{BUCKET}}"));
        assert!(!result.contains("{{CELL_ID}}"));
        assert!(result.contains("Test Dashboard"));
        assert!(result.contains("cpu"));
        assert!(result.contains("telegraf"));

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["dashboard"]["name"], "Test Dashboard");
        assert_eq!(parsed["dashboard"]["organization"], "default");
        assert_eq!(parsed["dashboard"]["cells"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_fill_template_multiple_measurements() {
        let config = ChronografDashboardConfig {
            name: "Multi Dashboard".to_string(),
            organization: "default".to_string(),
            measurements: vec![
                "cpu".to_string(),
                "mem".to_string(),
                "disk".to_string(),
                "net".to_string(),
            ],
            bucket: "telegraf".to_string(),
        };

        let template = load_template(None).unwrap();
        let result = fill_template(&template, &config).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let cells = parsed["dashboard"]["cells"].as_array().unwrap();

        assert_eq!(cells.len(), 4);

        // First row: cpu at (0,0), mem at (4,0), net at (8,0)
        assert_eq!(cells[0]["name"], "cpu");
        assert_eq!(cells[0]["x"], 0);
        assert_eq!(cells[0]["y"], 0);

        assert_eq!(cells[1]["name"], "mem");
        assert_eq!(cells[1]["x"], 4);
        assert_eq!(cells[1]["y"], 0);

        assert_eq!(cells[2]["name"], "disk");
        assert_eq!(cells[2]["x"], 8);
        assert_eq!(cells[2]["y"], 0);

        // Second row: net at (0,4)
        assert_eq!(cells[3]["name"], "net");
        assert_eq!(cells[3]["x"], 0);
        assert_eq!(cells[3]["y"], 4);
    }

    #[test]
    fn test_fill_template_empty_measurements() {
        let config = ChronografDashboardConfig {
            name: "Empty".to_string(),
            organization: "default".to_string(),
            measurements: vec![],
            bucket: "telegraf".to_string(),
        };

        let template = load_template(None).unwrap();
        let result = fill_template(&template, &config);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one measurement"));
    }

    #[test]
    fn test_generate_chronograf_dashboard_valid_json() {
        let config = ChronografDashboardConfig {
            name: "OPC UA Monitor".to_string(),
            organization: "default".to_string(),
            measurements: vec!["Sample_DB".to_string()],
            bucket: "telegraf".to_string(),
        };

        let result = generate_chronograf_dashboard(&config, None).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["dashboard"]["name"], "OPC UA Monitor");
        assert_eq!(parsed["meta"]["chronografVersion"], "1.10.9");
        assert_eq!(parsed["dashboard"]["cells"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_cell_positions_wrap_correctly() {
        let config = ChronografDashboardConfig {
            name: "Grid Test".to_string(),
            organization: "default".to_string(),
            measurements: vec![
                "m1".to_string(),
                "m2".to_string(),
                "m3".to_string(),
                "m4".to_string(),
                "m5".to_string(),
                "m6".to_string(),
                "m7".to_string(),
            ],
            bucket: "test".to_string(),
        };

        let template = load_template(None).unwrap();
        let result = fill_template(&template, &config).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let cells = parsed["dashboard"]["cells"].as_array().unwrap();

        // Row 0: (0,0) (4,0) (8,0)
        assert_eq!(cells[0]["x"], 0);
        assert_eq!(cells[0]["y"], 0);
        assert_eq!(cells[1]["x"], 4);
        assert_eq!(cells[1]["y"], 0);
        assert_eq!(cells[2]["x"], 8);
        assert_eq!(cells[2]["y"], 0);

        // Row 1: (0,4) (4,4) (8,4)
        assert_eq!(cells[3]["x"], 0);
        assert_eq!(cells[3]["y"], 4);
        assert_eq!(cells[4]["x"], 4);
        assert_eq!(cells[4]["y"], 4);
        assert_eq!(cells[5]["x"], 8);
        assert_eq!(cells[5]["y"], 4);

        // Row 2: (0,8)
        assert_eq!(cells[6]["x"], 0);
        assert_eq!(cells[6]["y"], 8);
    }

    #[test]
    fn test_flux_queries_reference_bucket_and_measurement() {
        let config = ChronografDashboardConfig {
            name: "Query Test".to_string(),
            organization: "default".to_string(),
            measurements: vec!["opcua_diagnostics".to_string()],
            bucket: "telegraf_diagnostics".to_string(),
        };

        let template = load_template(None).unwrap();
        let result = fill_template(&template, &config).unwrap();

        assert!(result.contains("telegraf_diagnostics"));
        assert!(result.contains("opcua_diagnostics"));
        assert!(result.contains("from(bucket:"));
        assert!(result.contains("|> filter(fn: (r) => r._measurement =="));
    }
}
