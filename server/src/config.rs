//! Server configuration — loaded from TOML file and/or environment variables.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Port to listen on.
    pub port: u16,
    /// Storage backend: "local", "s3", or "memory".
    pub storage: String,
    /// Local storage directory (when storage = "local").
    pub data_dir: String,
    /// Maximum upload size in bytes.
    #[allow(dead_code)]
    pub max_upload_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            storage: "local".to_string(),
            data_dir: "./data".to_string(),
            max_upload_size: 64 * 1024 * 1024, // 64MB
        }
    }
}

impl Config {
    /// Load configuration from file and environment variables.
    pub fn load() -> Self {
        // Try loading from s1.toml
        if let Ok(contents) = std::fs::read_to_string("s1.toml") {
            if let Ok(config) = toml::from_str(&contents) {
                return config;
            }
        }

        // Fall back to environment variables
        let mut config = Self::default();
        if let Ok(port) = std::env::var("S1_PORT") {
            if let Ok(p) = port.parse() {
                config.port = p;
            }
        }
        if let Ok(storage) = std::env::var("S1_STORAGE") {
            config.storage = storage;
        }
        if let Ok(dir) = std::env::var("S1_DATA_DIR") {
            config.data_dir = dir;
        }
        config
    }
}
