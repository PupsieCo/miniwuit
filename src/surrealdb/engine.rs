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
                convert_connection_result(
                    Surreal::new::<surrealdb::engine::local::Mem>(()).await
                ).map_err(|e| conduwuit::err!("{e}"))?
            }
            SurrealConnection::File { path } => {
                convert_connection_result(
                    Surreal::new::<surrealdb::engine::local::File>(path.clone()).await
                ).map_err(|e| conduwuit::err!("{e}"))?
            }
            SurrealConnection::Remote { url } => {
                Surreal::new::<Any>(url.as_str()).await?
            }
            SurrealConnection::TiKV { endpoints } => {
                let endpoint = endpoints.first()
                    .ok_or_else(|| conduwuit::err!("No TiKV endpoints provided"))?;
                Surreal::new::<surrealdb::engine::remote::tikv::TiKv>(endpoint).await?
            }
            SurrealConnection::FoundationDB { cluster_file } => {
                let path = cluster_file.as_ref()
                    .map(|p| p.to_string_lossy())
                    .unwrap_or_else(|| "".into());
                Surreal::new::<surrealdb::engine::remote::fdb::Fdb>(path.as_ref()).await?
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
    pub async fn query(&self, sql: &str) -> Result<Vec<Response>> {
        debug!("Executing query: {}", sql);
        let result = convert_surreal_error(self.db.query(sql).await)
            .map_err(|e| conduwuit::err!("{e}"))?;
        // TODO: Proper handling if query fails
        Ok(vec![result])
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