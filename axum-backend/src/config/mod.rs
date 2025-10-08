/// Configuration management for Mechaway engine
/// 
/// Handles server configuration, database connections, and runtime parameters.

use serde::{Deserialize, Serialize};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration  
    pub database: DatabaseConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address (e.g., "0.0.0.0")
    pub host: String,
    /// Server port number
    pub port: u16,
}

/// Database configuration for project-isolated storage
#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct DatabaseConfig {
    /// Base directory for all project databases (default: "data")
    /// Creates: {project_slug}_project.db, {project_slug}_simpletable.db
    pub project_data_dir: String,
}

impl Default for Config {
    /// Default configuration with ENV_VAR support for k8s/container deployment
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: std::env::var("MECHAWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("MECHAWAY_PORT")
                    .unwrap_or_else(|_| "3004".to_string())
                    .parse()
                    .unwrap_or(3004),
            },
            database: DatabaseConfig {
                project_data_dir: std::env::var("MECHAWAY_DATA_DIR")
                    .unwrap_or_else(|_| "data".to_string()),
            },
        }
    }
}
