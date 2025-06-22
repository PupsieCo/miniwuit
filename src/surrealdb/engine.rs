use std::sync::Arc;
use surrealdb::{Surreal, engine::any::Any, sql::Value, Response};
use conduwuit::{Result, error, debug, info, warn}; 

use crate::{
    config::{SurrealConfig, SurrealConnection},
    context::Context,
    pool::ConnectionPool,
    error::{Error, convert_surreal_error, convert_connection_result},
};

pub struct Engine {
    pub db: Surreal<Any>,
    pub ctx: Arc<Context>,
    pub pool: Arc<ConnectionPool>,
    connection_mode: SurrealConnection,
}

impl Engine {
    pub async fn open(ctx: Arc<Context>) -> Result<Arc<Self>> {
        info!("Opening SurrealDB engine");
        debug!("Connection mode: {:?}", ctx.config.connection);
        let db = match &ctx.config.connection {
            SurrealConnection::Memory => {
                Surreal::new::<surrealdb::engine::local::Mem>(()).await
                    .map_err(|e| conduwuit::err!("Failed to create memory connection: {e}"))?
                    .into()
            }
            SurrealConnection::File { path } => {
                Surreal::new::<surrealdb::engine::local::File>(path.clone()).await
                    .map_err(|e| conduwuit::err!("Failed to create file connection: {e}"))?
                    .into()
            }
            SurrealConnection::Remote { url } => {
                Surreal::new::<surrealdb::engine::remote::ws::Ws>(url.as_str()).await
                    .map_err(|e| conduwuit::err!("Failed to create remote connection: {e}"))?
                    .into()
            }
            SurrealConnection::TiKV { endpoints: _ } => {
                return Err(conduwuit::err!("TiKV connection not supported in this SurrealDB build"));
            }
            SurrealConnection::FoundationDB { cluster_file: _ } => {
                return Err(conduwuit::err!("FoundationDB connection not supported in this SurrealDB build"));
            }
        };

        info!("SurrealDB Database initialized");

        // Authenticate if credentials provided
        if let Some(ref auth) = ctx.config.auth.token {
            info!("Authenticating with token");
            db.authenticate(auth).await?;
        } else if let (Some(ref user), Some(ref pass)) = 
            (&ctx.config.auth.username, &ctx.config.auth.password) {
            info!("Authenticating with username and password");
            db.signin(surrealdb::opt::auth::Root {
                username: user,
                password: pass,
            }).await?;
        }
        // TODO: Proper handling if authentication fails
        
        // Select namespace and database
        info!("Selecting namespace and database");
        db.use_ns(&ctx.config.namespace)
          .use_db(&ctx.config.database)
          .await?;
        // TODO: Proper handling if namespace or database selection fails
          
        info!("SurrealDB engine initialized");
        Ok(Arc::new(Self {
            db,
            ctx,
            pool: ctx.pool.clone(),
            connection_mode: ctx.config.connection.clone(),
        }))
    }
    
    /// Execute a query with automatic connection management
    pub async fn query(&self, sql: &str) -> Result<surrealdb::Response> {
        debug!("Executing query: {}", sql);
        let result = self.db.query(sql).await?;
        // TODO: Proper handling if query fails
        Ok(result)
    }
    
    /// Update database schema
    pub async fn update_schema(&self, schema_sql: &str) -> Result<()> {
        debug!("Updating schema: {}", schema_sql);
        self.query(schema_sql).await?;
        // TODO: Proper handling if schema update fails
        Ok(())
    }
    
    /// Health check
    pub async fn ping(&self) -> Result<()> {
        info!("Pinging SurrealDB engine");
        convert_surreal_error(self.db.health().await)
            .map_err(|e| conduwuit::err!("{e}"))?;
        // TODO: Proper handling if health check fails
        Ok(())
    }
}