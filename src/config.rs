use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_ssh_command")]
    pub ssh_command: String,
    #[serde(default = "default_max_log_lines")]
    pub max_log_lines: usize,
    #[serde(default = "default_discovery_timeout")]
    pub discovery_timeout: u64,
    #[serde(default)]
    pub extra_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ssh_command: default_ssh_command(),
            max_log_lines: default_max_log_lines(),
            discovery_timeout: default_discovery_timeout(),
            extra_paths: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!(
                            "Warning: failed to parse config {}: {}. Using defaults.",
                            path.display(),
                            e
                        );
                        Self::default()
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Warning: failed to read config {}: {}. Using defaults.",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    pub fn config_path() -> PathBuf {
        match ProjectDirs::from("com", "janvete", "lview") {
            Some(dirs) => dirs.config_dir().join("config.toml"),
            None => PathBuf::from(".").join("lview-config.toml"),
        }
    }
}

fn default_ssh_command() -> String {
    "ssh".to_string()
}

fn default_max_log_lines() -> usize {
    10_000
}

fn default_discovery_timeout() -> u64 {
    10
}
