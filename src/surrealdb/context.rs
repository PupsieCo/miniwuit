use std::sync::Arc;
use conduwuit::{Result, Server};
use surrealdb::{Surreal, engine::any::Any};

use crate::{
    config::SurrealConfig,
    pool::ConnectionPool,
};

pub struct Context {
    pub server: Arc<Server>,
    pub config: SurrealConfig,
    pub pool: Arc<ConnectionPool>,
}

impl Context {
    pub async fn new(server: &Arc<Server>) -> Result<Arc<Self>> {
        let config = Self::extract_config(server)?;
        let pool = ConnectionPool::new(&config).await?;
        
        Ok(Arc::new(Self {
            server: server.clone(),
            config,
            pool,
        }))
    }
    
    fn extract_config(server: &Arc<Server>) -> Result<SurrealConfig> {
        // Extract SurrealDB config from server config
        // This would integrate with conduwuit's config system
        // For now, return a default configuration
        use crate::config::{SurrealConnection, SurrealAuthConfig, SurrealPoolConfig, SurrealCapabilities};
        
        Ok(SurrealConfig {
            connection: SurrealConnection::Memory,
            namespace: "conduwuit".to_string(),
            database: "main".to_string(),
            auth: SurrealAuthConfig {
                username: None,
                password: None,
                token: None,
            },
            pool: SurrealPoolConfig {
                max_connections: 10,
                connection_timeout: 30,
                idle_timeout: 300,
            },
            query_timeout: 30,
            transaction_timeout: 60,
            strict_mode: false,
            capabilities: SurrealCapabilities {
                allow_functions: true,
                allow_network: false,
                allow_scripting: false,
                allow_guests: false,
            },
        })
    }
}