extern crate conduwuit_core as conduwuit;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}

// mod config;
// mod context;
// mod engine;
// mod pool;
// mod service;
mod connection;
// mod queries;
mod error;

#[cfg(test)]
mod tests;

// pub use self::{
//     // engine::Engine,
//     // context::Context,
//     // service::Service,
//     connection::{SurrealConnection, SurrealConnectionStats}, // SurrealConnectionFactory, SurrealHealthChecker,
//     // pool::{ConnectionPool, PooledConnection},
//     // queries::{QueryBuilder, TableSchema, FieldDefinition, IndexDefinition},
//     error::{Error, Result},
// };

pub use connection::{SurrealConnection, SurrealConnectionStats};
pub use error::{Error, Result};

use std::sync::Arc;
use conduwuit::{Result as CoreResult, Server, debug, info, warn};
use conduwuit_core::config::surrealdb::{SurrealConfig, SurrealConnectionConfig, SurrealAuthConfig, SurrealPoolConfig, SurrealCapabilities};

pub struct SurrealDatabase {
    pub server: Arc<Server>,
    pub config: SurrealConfig,
    //pub pool: Arc<ConnectionPool>,
    pub engine: Option<SurrealConnection>,
}

impl SurrealDatabase {
    pub fn new(server: &Arc<Server>) -> Result<Arc<Self>> {

        // let server = &self.ctx.server;
        // let config = &server.config.surrealdb;
        // let config = Self::extract_config(server)?;
        // let pool = ConnectionPool::new(config.clone()).await?;
        // let connection = SurrealConnection::new(config).await?;
        
        Ok(Arc::new(Self {
            server: server.clone(),
            config: server.config.surrealdb.clone(),
            engine: None,
        }))
    }
    /// Initialize SurrealDB following conduwuit patterns
    pub async fn open(&mut self) -> CoreResult<Arc<Self>> {
        info!("Opening SurrealDB");
        // let ctx = Context::new(server).await?;
        // let engine = Engine::open(ctx.clone()).await?;
        
        let engine = SurrealConnection::new(self.config).await?;

        self.engine = Some(engine);
        info!("SurrealDB opened");
        Ok(engine)
        // Ok(Arc::new(Self {
        //     server: self.server.clone(),
        //     config: self.config.clone(),
        //     engine: Option::<engine>,
        // }))
    }
}