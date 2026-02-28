use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_static_dir")]
    pub static_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_url")]
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default = "default_expected_status")]
    pub expected_status: u16,
    pub check_interval: Option<u64>,
    pub timeout: Option<u64>,
}

fn default_port() -> u16 {
    8080
}

fn default_static_dir() -> String {
    "./frontend/dist".to_string()
}

fn default_check_interval() -> u64 {
    60
}

fn default_timeout() -> u64 {
    10
}

fn default_db_url() -> String {
    "sqlite://data/tickers.db".to_string()
}

fn default_expected_status() -> u16 {
    200
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            static_dir: default_static_dir(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            check_interval: default_check_interval(),
            timeout: default_timeout(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_db_url(),
        }
    }
}

impl ServiceConfig {
    pub fn effective_check_interval(&self, defaults: &DefaultsConfig) -> u64 {
        self.check_interval.unwrap_or(defaults.check_interval)
    }

    pub fn effective_timeout(&self, defaults: &DefaultsConfig) -> u64 {
        self.timeout.unwrap_or(defaults.timeout)
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path.as_ref())?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        if path.as_ref().exists() {
            Self::load(path)
        } else {
            tracing::warn!("Config file not found, using defaults");
            Ok(Config {
                server: ServerConfig::default(),
                defaults: DefaultsConfig::default(),
                database: DatabaseConfig::default(),
                services: vec![],
            })
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let mut seen = std::collections::HashSet::new();
        for svc in &self.services {
            if !seen.insert(&svc.id) {
                return Err(ConfigError::DuplicateServiceId(svc.id.clone()));
            }
            if svc.url.parse::<url::Url>().is_err() {
                return Err(ConfigError::InvalidUrl(svc.id.clone(), svc.url.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Duplicate service ID: {0}")]
    DuplicateServiceId(String),
    #[error("Invalid URL for service '{0}': {1}")]
    InvalidUrl(String, String),
}
