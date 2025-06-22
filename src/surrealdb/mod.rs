extern crate conduwuit_core as conduwuit;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}

mod config;
mod context;
mod engine;
mod pool;
mod service;
mod connection;
mod queries;
mod error;

#[cfg(test)]
mod tests;

pub use self::{
    config::{SurrealConfig, SurrealConnection, SurrealAuthConfig, SurrealPoolConfig, SurrealCapabilities},
    engine::Engine,
    context::Context,
    service::Service,
    connection::{Connection, ConnectionFactory, HealthChecker, ConnectionStats},
    pool::{ConnectionPool, PooledConnection},
    queries::{QueryBuilder, TableSchema, FieldDefinition, IndexDefinition},
    error::{Error, Result},
};

use std::sync::Arc;
use conduwuit::{Result as CoreResult, Server, debug, info, warn};

pub struct SurrealDB {
    pub engine: Arc<Engine>,
    pub ctx: Arc<Context>,
}

impl SurrealDB {
    /// Initialize SurrealDB following conduwuit patterns
    pub async fn open(server: &Arc<Server>) -> CoreResult<Arc<Self>> {
        info!("Opening SurrealDB");
        let ctx = Context::new(server).await?;
        let engine = Engine::open(ctx.clone()).await?;
        
        info!("SurrealDB opened");
        Ok(Arc::new(Self {
            engine,
            ctx,
        }))
    }
    
    /// Execute query with error handling
    pub async fn query(&self, sql: &str) -> CoreResult<Vec<surrealdb::Response>> {
        self.engine.query(sql).await
            .map_err(|e| conduwuit::err!("SurrealDB query failed: {e}"))
    }
}