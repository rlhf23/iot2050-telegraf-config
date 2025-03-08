use crate::backend::ssh_utils;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    // Helper function to create a test file with content
    fn create_test_file(dir: &PathBuf, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    // We can't easily test actual SSH connections in unit tests
    // So we'll test the error handling for missing files and invalid paths
    
    #[test]
    fn test_send_and_restart_telegraf_missing_file() {
        let non_existent_path = PathBuf::from("/this/file/does/not/exist.conf");
        let result = ssh_utils::send_and_restart_telegraf(
            &non_existent_path,
            "/remote/path",
            "192.168.1.100:22",
            "user",
            "pass",
        );
        
        assert!(result.is_err());
        
        // Extract the error and check it's related to file not found
        let err = result.unwrap_err();
        let err_string = format!("{}", err);
        assert!(err_string.contains("No such file") || err_string.contains("not found") || 
                err_string.contains("cannot find"));
    }
    
    #[test]
    fn test_send_file_over_ssh_invalid_host() {
        let dir = tempdir().unwrap();
        let file_path = create_test_file(&dir.path().to_path_buf(), "test.conf", "test content");
        
        // Use an invalid host that will fail to connect (port 1 is unlikely to have SSH)
        let result = ssh_utils::send_file_over_ssh(
            &file_path,
            "/remote/path",
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_backup_influxdb_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::backup_influxdb(
            "127.0.0.1:1", // Invalid port
            "user", 
            "pass",
            "token",
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_backup_grafana_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::backup_grafana_config(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_get_telegraf_status_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::get_telegraf_status(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_get_telegraf_logs_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::get_telegraf_logs(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
            10,
        );
        
        assert!(result.is_err());
    }
}
