// Tests for backup.rs

use super::backup::{
    extract_backup_name_from_archive, extract_top_dir_from_tar_listing, generate_backup_name,
    parse_datasource_name, parse_grafana_credentials, parse_influxdb_bucket_list,
    parse_influxdb_credentials, transform_grafana_dashboard_for_import,
};

// ============================================================================
// Grafana dashboard transformation tests
// ============================================================================

#[test]
fn transform_grafana_simple_dashboard() {
    let export_json =
        r#"{"dashboard": {"id": 1, "title": "My Dashboard"}, "meta": {"isStarred": false}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("dashboard").is_some());
    assert!(parsed.get("overwrite").is_some());
    assert_eq!(parsed["overwrite"], true);
    assert!(parsed["dashboard"]["title"] == "My Dashboard");
    assert!(parsed.get("meta").is_none(), "meta should be removed");
    assert!(
        !parsed["dashboard"].as_object().unwrap().contains_key("id"),
        "id should be removed"
    );
}

#[test]
fn transform_grafana_removes_id_and_version() {
    // This is the critical test - ensures id and version are removed
    let export_json =
        r#"{"dashboard": {"id": 47, "uid": "abc123", "version": 12, "title": "Test"}, "meta": {}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let dashboard = &parsed["dashboard"];

    // id and version must NOT be present
    assert!(
        !dashboard.as_object().unwrap().contains_key("id"),
        "id should be removed for import"
    );
    assert!(
        !dashboard.as_object().unwrap().contains_key("version"),
        "version should be removed for import"
    );

    // uid and other fields should remain
    assert!(dashboard["uid"] == "abc123", "uid should be preserved");
    assert!(dashboard["title"] == "Test", "title should be preserved");
}

#[test]
fn transform_grafana_preserves_uid() {
    let export_json =
        r#"{"dashboard": {"id": 99, "uid": "my-custom-uid", "title": "Dashboard"}, "meta": {}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(
        parsed["dashboard"]["uid"] == "my-custom-uid",
        "uid should be preserved for dashboard identity"
    );
    assert!(!parsed["dashboard"].as_object().unwrap().contains_key("id"));
}

#[test]
fn transform_grafana_nested_dashboard() {
    let export_json = r#"{"dashboard": {"id": 1, "panels": [{"title": "Panel1"}, {"title": "Panel2"}]}, "meta": {}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["dashboard"]["panels"].is_array());
    assert!(parsed.get("meta").is_none());
    assert_eq!(parsed["overwrite"], true);
    assert!(
        !parsed["dashboard"].as_object().unwrap().contains_key("id"),
        "id should be removed"
    );
}

#[test]
fn transform_grafana_empty_dashboard() {
    let export_json = r#"{"dashboard": {}, "meta": {}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["dashboard"], serde_json::json!({}));
    assert_eq!(parsed["overwrite"], true);
}

#[test]
fn transform_grafana_no_dashboard_key() {
    let no_dashboard_key = r#"{"id": 1, "title": "Something"}"#;
    let result = transform_grafana_dashboard_for_import(no_dashboard_key);

    // Should return error when no dashboard key exists
    assert!(
        result.is_err(),
        "Should return error when dashboard key is missing"
    );
}

#[test]
fn transform_grafana_with_whitespace() {
    let export_json = r#"
    {
        "dashboard": {
            "id": 1,
            "title": "Test"
        },
        "meta": {}
    }
    "#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("dashboard").is_some());
    assert_eq!(parsed["overwrite"], true);
    assert!(
        !parsed["dashboard"].as_object().unwrap().contains_key("id"),
        "id should be removed"
    );
}

#[test]
fn transform_grafana_complex_nested_structure() {
    let export_json = r#"{"dashboard": {"id": 42, "uid": "abc123", "version": 5, "nested": {"deep": {"value": 42}}}, "meta": {"created": "2024-01-01"}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("meta").is_none());
    assert!(parsed["dashboard"]["uid"] == "abc123");
    assert!(parsed["dashboard"]["nested"]["deep"]["value"] == 42);
    assert!(
        !parsed["dashboard"].as_object().unwrap().contains_key("id"),
        "id should be removed"
    );
    assert!(
        !parsed["dashboard"]
            .as_object()
            .unwrap()
            .contains_key("version"),
        "version should be removed"
    );
}

#[test]
fn transform_grafana_preserves_arrays() {
    let export_json =
        r#"{"dashboard": {"id": 1, "tags": ["tag1", "tag2"], "panels": [1, 2, 3]}, "meta": {}}"#;
    let result = transform_grafana_dashboard_for_import(export_json).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["dashboard"]["tags"].is_array());
    assert!(parsed["dashboard"]["panels"].is_array());
    assert!(
        !parsed["dashboard"].as_object().unwrap().contains_key("id"),
        "id should be removed"
    );
}

#[test]
fn transform_grafana_invalid_json() {
    let invalid_json = r#"{"dashboard": {invalid json}"#;
    let result = transform_grafana_dashboard_for_import(invalid_json);
    assert!(result.is_err(), "Should return error for invalid JSON");
}

// ============================================================================
// Backup name generation tests
// ============================================================================

#[test]
fn generate_backup_name_format() {
    let name = generate_backup_name();

    assert!(name.starts_with("monitoring_backup_"));

    let timestamp_part = name.strip_prefix("monitoring_backup_").unwrap();
    let parts: Vec<&str> = timestamp_part.split('_').collect();
    assert_eq!(parts.len(), 2, "Should have date and time parts");

    assert_eq!(parts[0].len(), 8, "Date part should be 8 characters");
    assert!(
        parts[0].chars().all(|c| c.is_numeric()),
        "Date should be numeric"
    );

    assert_eq!(parts[1].len(), 6, "Time part should be 6 characters");
    assert!(
        parts[1].chars().all(|c| c.is_numeric()),
        "Time should be numeric"
    );
}

#[test]
fn extract_backup_name_simple() {
    let result = extract_backup_name_from_archive("/tmp/monitoring_backup_20240101_120000.tar.gz");
    assert_eq!(
        result,
        Some("monitoring_backup_20240101_120000".to_string())
    );
}

#[test]
fn extract_backup_name_current_dir() {
    let result = extract_backup_name_from_archive("monitoring_backup_20241225_235959.tar.gz");
    assert_eq!(
        result,
        Some("monitoring_backup_20241225_235959".to_string())
    );
}

#[test]
fn extract_backup_name_nested_path() {
    let result = extract_backup_name_from_archive(
        "/home/user/backups/monitoring_backup_20230115_100530.tar.gz",
    );
    assert_eq!(
        result,
        Some("monitoring_backup_20230115_100530".to_string())
    );
}

#[test]
fn extract_backup_name_no_extension() {
    let result = extract_backup_name_from_archive("monitoring_backup_20240101_120000");
    assert_eq!(result, None);
}

#[test]
fn extract_backup_name_wrong_extension() {
    let result = extract_backup_name_from_archive("monitoring_backup_20240101_120000.zip");
    assert_eq!(result, None);
}

#[test]
fn extract_backup_name_empty() {
    let result = extract_backup_name_from_archive("");
    assert_eq!(result, None);
}

// ============================================================================
// Grafana credentials parsing tests
// ============================================================================

#[test]
fn parse_grafana_credentials_both() {
    let creds = "GRAFANA_ADMIN_USER=admin\nGRAFANA_ADMIN_PASSWORD=secret123\n";
    let (user, pass) = parse_grafana_credentials(creds);
    assert_eq!(user, "admin");
    assert_eq!(pass, "secret123");
}

#[test]
fn parse_grafana_credentials_only_user() {
    let creds = "GRAFANA_ADMIN_USER=myuser\n";
    let (user, pass) = parse_grafana_credentials(creds);
    assert_eq!(user, "myuser");
    assert_eq!(pass, "admin");
}

#[test]
fn parse_grafana_credentials_only_password() {
    let creds = "GRAFANA_ADMIN_PASSWORD=mypassword\n";
    let (user, pass) = parse_grafana_credentials(creds);
    assert_eq!(user, "admin");
    assert_eq!(pass, "mypassword");
}

#[test]
fn parse_grafana_credentials_empty() {
    let creds = "";
    let (user, pass) = parse_grafana_credentials(creds);
    assert_eq!(user, "admin");
    assert_eq!(pass, "admin");
}

#[test]
fn parse_grafana_credentials_with_other_vars() {
    let creds = "SOME_VAR=value\nGRAFANA_ADMIN_USER=grafana_user\nOTHER_VAR=other\nGRAFANA_ADMIN_PASSWORD=grafana_pass\n";
    let (user, pass) = parse_grafana_credentials(creds);
    assert_eq!(user, "grafana_user");
    assert_eq!(pass, "grafana_pass");
}

#[test]
fn parse_grafana_credentials_equals_in_value() {
    let creds = "GRAFANA_ADMIN_PASSWORD=password=with=equals\n";
    let (user, pass) = parse_grafana_credentials(creds);
    assert_eq!(user, "admin");
    assert_eq!(pass, "password=with=equals");
}

// ============================================================================
// InfluxDB credentials parsing tests
// ============================================================================

#[test]
fn parse_influxdb_credentials_both() {
    let env = "INFLUXDB_TOKEN=mytoken123\nINFLUXDB_ORG=myorg\n";
    let (token, org) = parse_influxdb_credentials(env);
    assert_eq!(token, Some("mytoken123".to_string()));
    assert_eq!(org, Some("myorg".to_string()));
}

#[test]
fn parse_influxdb_credentials_only_token() {
    let env = "INFLUXDB_TOKEN=secrettoken\n";
    let (token, org) = parse_influxdb_credentials(env);
    assert_eq!(token, Some("secrettoken".to_string()));
    assert_eq!(org, None);
}

#[test]
fn parse_influxdb_credentials_only_org() {
    let env = "INFLUXDB_ORG=myorganization\n";
    let (token, org) = parse_influxdb_credentials(env);
    assert_eq!(token, None);
    assert_eq!(org, Some("myorganization".to_string()));
}

#[test]
fn parse_influxdb_credentials_empty() {
    let env = "";
    let (token, org) = parse_influxdb_credentials(env);
    assert_eq!(token, None);
    assert_eq!(org, None);
}

#[test]
fn parse_influxdb_credentials_with_other_vars() {
    let env =
        "OTHER_VAR=value\nINFLUXDB_TOKEN=token123\nANOTHER_VAR=another\nINFLUXDB_ORG=testorg\n";
    let (token, org) = parse_influxdb_credentials(env);
    assert_eq!(token, Some("token123".to_string()));
    assert_eq!(org, Some("testorg".to_string()));
}

// ============================================================================
// InfluxDB bucket list parsing tests
// ============================================================================

#[test]
fn parse_bucket_list_simple() {
    let output =
        "ID\tName\tRetention Policy\nabc123\tmy_bucket\tinfinite\ndef456\tanother_bucket\t30d";
    let buckets = parse_influxdb_bucket_list(output);
    assert_eq!(buckets, vec!["my_bucket", "another_bucket"]);
}

#[test]
fn parse_bucket_list_excludes_system_buckets() {
    let output = "ID\tName\tRetention\nabc123\t_tasks\tinfinite\ndef456\tmy_data\t30d\nghi789\t_buckets\tinfinite";
    let buckets = parse_influxdb_bucket_list(output);
    assert_eq!(buckets, vec!["my_data"]);
}

#[test]
fn parse_bucket_list_empty() {
    let output = "ID\tName\tRetention\n";
    let buckets = parse_influxdb_bucket_list(output);
    assert!(buckets.is_empty());
}

#[test]
fn parse_bucket_list_single_bucket() {
    let output = "ID\tName\tRetention\nabc123\tsingle_bucket\tinfinite";
    let buckets = parse_influxdb_bucket_list(output);
    assert_eq!(buckets, vec!["single_bucket"]);
}

#[test]
fn parse_bucket_list_with_underscores_in_name() {
    let output = "ID\tName\nabc\tmy_long_bucket_name\t7d";
    let buckets = parse_influxdb_bucket_list(output);
    assert_eq!(buckets, vec!["my_long_bucket_name"]);
}

// ============================================================================
// Datasource name parsing tests
// ============================================================================

#[test]
fn parse_datasource_name_simple() {
    let line = r#"{"id":1,"name":"InfluxDB","type":"influxdb"}"#;
    let result = parse_datasource_name(line);
    assert_eq!(result, Some("InfluxDB".to_string()));
}

#[test]
fn parse_datasource_name_with_spaces() {
    let line = r#"{"id":2,"name":"My Data Source","type":"prometheus"}"#;
    let result = parse_datasource_name(line);
    assert_eq!(result, Some("My Data Source".to_string()));
}

#[test]
fn parse_datasource_name_not_json_object() {
    let line = r#"[{"id":1,"name":"test"}]"#;
    let result = parse_datasource_name(line);
    assert_eq!(result, None);
}

#[test]
fn parse_datasource_name_empty() {
    let line = "";
    let result = parse_datasource_name(line);
    assert_eq!(result, None);
}

#[test]
fn parse_datasource_name_whitespace() {
    let line = r#"  {"id":1,"name":"  trimmed  ","type":"test"}  "#;
    let result = parse_datasource_name(line);
    assert_eq!(result, Some("  trimmed  ".to_string()));
}

#[test]
fn parse_datasource_name_missing_name() {
    let line = r#"{"id":1,"type":"influxdb"}"#;
    let result = parse_datasource_name(line);
    assert_eq!(result, None);
}

#[test]
fn parse_datasource_name_complex_json() {
    let line = r#"{"id":42,"name":"Production InfluxDB","type":"influxdb","url":"http://localhost:8086","access":"proxy","isDefault":true}"#;
    let result = parse_datasource_name(line);
    assert_eq!(result, Some("Production InfluxDB".to_string()));
}

// ============================================================================
// Tar listing parsing tests (top-level directory discovery)
// ============================================================================

#[test]
fn extract_top_dir_from_typical_tar_listing() {
    let listing = "monitoring_backup_20260421_130407/\nmonitoring_backup_20260421_130407/influxdb/\nmonitoring_backup_20260421_130407/grafana/\n";
    let result = extract_top_dir_from_tar_listing(listing);
    assert_eq!(result, Some("monitoring_backup_20260421_130407".to_string()));
}

#[test]
fn extract_top_dir_without_trailing_slash() {
    let listing = "monitoring_backup_20240101_120000\nmonitoring_backup_20240101_120000/influxdb\n";
    let result = extract_top_dir_from_tar_listing(listing);
    assert_eq!(result, Some("monitoring_backup_20240101_120000".to_string()));
}

#[test]
fn extract_top_dir_renamed_archive() {
    let listing = "monitoring_backup_20260421_130407/\nmonitoring_backup_20260421_130407/influxdb/\n";
    let result = extract_top_dir_from_tar_listing(listing);
    assert_eq!(result, Some("monitoring_backup_20260421_130407".to_string()));
}

#[test]
fn extract_top_dir_empty() {
    let result = extract_top_dir_from_tar_listing("");
    assert_eq!(result, None);
}

#[test]
fn extract_top_dir_just_slash() {
    let result = extract_top_dir_from_tar_listing("/");
    assert_eq!(result, None);
}

#[test]
fn extract_top_dir_single_dir() {
    let listing = "my_backup/";
    let result = extract_top_dir_from_tar_listing(listing);
    assert_eq!(result, Some("my_backup".to_string()));
}

#[test]
fn extract_top_dir_with_leading_dot_slash() {
    let listing = "./monitoring_backup_20260421_130407/\n./monitoring_backup_20260421_130407/influxdb/\n";
    let result = extract_top_dir_from_tar_listing(listing);
    assert_eq!(result, Some(".".to_string()));
}

#[test]
fn extract_top_dir_deep_nested_first_line() {
    let listing = "a/b/c/d\n";
    let result = extract_top_dir_from_tar_listing(listing);
    assert_eq!(result, Some("a".to_string()));
}

// ============================================================================
// Integration: create real tar.gz and verify tar listing parsing
// ============================================================================

#[test]
fn real_tar_renamed_archive_discovers_correct_dir() {
    use std::fs;
    use std::process::Command;

    let tmp = std::env::temp_dir().join("backup_test_renamed_archive");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let backup_dir = tmp.join("monitoring_backup_20260421_130407");
    fs::create_dir_all(backup_dir.join("influxdb")).unwrap();
    fs::create_dir_all(backup_dir.join("grafana/dashboards")).unwrap();
    fs::write(backup_dir.join("influxdb/meta"), "test").unwrap();
    fs::write(backup_dir.join("grafana/dashboards/test.json"), "{}").unwrap();

    let original_archive = tmp.join("monitoring_backup_20260421_130407.tar.gz");
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&original_archive)
        .arg("-C")
        .arg(&tmp)
        .arg("monitoring_backup_20260421_130407")
        .current_dir(&tmp)
        .status()
        .unwrap();
    assert!(status.success(), "tar creation failed");

    let renamed_archive = tmp.join("2653_monitoring_backup_20260421_130407.tar.gz");
    fs::copy(&original_archive, &renamed_archive).unwrap();

    let output = Command::new("tar")
        .arg("-tzf")
        .arg(&renamed_archive)
        .output()
        .unwrap();
    let listing = String::from_utf8_lossy(&output.stdout);
    let result = extract_top_dir_from_tar_listing(&listing);

    assert_eq!(
        result,
        Some("monitoring_backup_20260421_130407".to_string()),
        "Should discover 'monitoring_backup_20260421_130407' from renamed archive, not '2653_monitoring_backup_20260421_130407'"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn real_tar_original_archive_discovers_correct_dir() {
    use std::fs;
    use std::process::Command;

    let tmp = std::env::temp_dir().join("backup_test_original_archive");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let backup_dir = tmp.join("monitoring_backup_20240101_120000");
    fs::create_dir_all(backup_dir.join("influxdb")).unwrap();
    fs::write(backup_dir.join("influxdb/meta"), "test").unwrap();

    let archive = tmp.join("monitoring_backup_20240101_120000.tar.gz");
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&tmp)
        .arg("monitoring_backup_20240101_120000")
        .current_dir(&tmp)
        .status()
        .unwrap();
    assert!(status.success(), "tar creation failed");

    let output = Command::new("tar")
        .arg("-tzf")
        .arg(&archive)
        .output()
        .unwrap();
    let listing = String::from_utf8_lossy(&output.stdout);
    let result = extract_top_dir_from_tar_listing(&listing);

    assert_eq!(
        result,
        Some("monitoring_backup_20240101_120000".to_string()),
        "Should discover correct dir from original (non-renamed) archive"
    );

    let _ = fs::remove_dir_all(&tmp);
}
