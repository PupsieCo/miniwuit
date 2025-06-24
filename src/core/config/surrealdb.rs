use serde::{Deserialize, Serialize};
use url::Url;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SurrealConfig {
    /// Connection mode: "memory", "file", "tikv", "foundationdb", or remote URL
    #[serde(default = "default_connection")]
    pub connection: SurrealConnectionConfig,
    
    /// Database namespace
    pub namespace: String,
    
    /// Database name  
    pub database: String,
    
    /// Authentication configuration
    #[serde(default)]
    pub auth: SurrealAuthConfig,
    
    /// Connection pool configuration
    #[serde(default)]
    pub pool: SurrealPoolConfig,
    
    /// Query timeout in seconds
    #[serde(default = "default_query_timeout")]
    pub query_timeout: u64,
    
    /// Transaction timeout in seconds
    #[serde(default = "default_transaction_timeout")]  
    pub transaction_timeout: u64,
    
    /// Enable strict mode
    #[serde(default)]
    pub strict_mode: bool,
    
    /// Capabilities configuration
    #[serde(default)]
    pub capabilities: SurrealCapabilities,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode")]
pub enum SurrealConnectionConfig {
    Memory,
    File { path: PathBuf },
    RocksDb { path: PathBuf },
    Remote { url: Url }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SurrealAuthConfig {
    /// Root username (for embedded mode)
    pub username: Option<String>,
    /// Root password (for embedded mode) 
    pub password: Option<String>,
    /// JWT token for remote connections
    pub token: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SurrealPoolConfig {
    /// Maximum number of connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
    /// Idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SurrealCapabilities {
    #[serde(default = "default_true")]
    pub allow_functions: bool,
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default)]
    pub allow_scripting: bool,
    #[serde(default)]
    pub allow_guests: bool,
}

// Default value functions
fn default_connection() -> SurrealConnectionConfig {
    SurrealConnectionConfig::Memory
}

fn default_query_timeout() -> u64 {
    30
}

fn default_transaction_timeout() -> u64 {
    60
}

fn default_max_connections() -> usize {
    10
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_idle_timeout() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

impl Default for SurrealConfig {
    fn default() -> Self {
        Self {
            connection: default_connection(),
            namespace: "conduwuit".to_string(),
            database: "main".to_string(),
            auth: SurrealAuthConfig::default(),
            pool: SurrealPoolConfig::default(),
            query_timeout: default_query_timeout(),
            transaction_timeout: default_transaction_timeout(),
            strict_mode: false,
            capabilities: SurrealCapabilities::default(),
        }
    }
}

impl Default for SurrealAuthConfig {
    fn default() -> Self {
        Self {
            username: None,
            password: None,
            token: None,
        }
    }
}

impl Default for SurrealPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            connection_timeout: default_connection_timeout(),
            idle_timeout: default_idle_timeout(),
        }
    }
}

impl Default for SurrealCapabilities {
    fn default() -> Self {
        Self {
            allow_functions: default_true(),
            allow_network: false,
            allow_scripting: false,
            allow_guests: false,
        }
    }
}