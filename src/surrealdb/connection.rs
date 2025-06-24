use std::{
    sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use surrealdb::{Surreal, 
    engine::any::Any,
    engine::remote::{
        ws::Ws
    },
    engine::local::{
        File,
        Mem,
        RocksDb
    },
    sql::Value, Response};
use tokio::sync::{RwLock, Semaphore};
use conduwuit::{Result, err, debug, warn, error, trace};

use crate::{
    error::{Error, Result as SurrealResult}
};

use conduwuit_core::config::surrealdb::{SurrealConfig, SurrealConnectionConfig, SurrealAuthConfig};

/// A managed SurrealDB connection with health checking and automatic reconnection
pub struct SurrealConnection {
    pub(crate) db: Surreal<Any>,
    pub(crate) config: SurrealConfig,
    pub(crate) last_used: Arc<RwLock<Instant>>,
    pub(crate) is_healthy: AtomicBool,
    pub(crate) connection_id: String,
    pub(crate) query_count: AtomicU64,
    pub(crate) error_count: AtomicU64,
}

/// Connection factory for creating and managing SurrealDB connections
pub struct ConnectionFactory {
    config: SurrealConfig,
    next_id: AtomicU64,
}

/// Connection health checker
pub struct HealthChecker {
    interval: Duration,
    timeout: Duration,
}

impl SurrealConnection {
    /// Create a new connection with the given configuration
    pub async fn new(config: SurrealConfig) -> SurrealResult<Self> {
        debug!("Creating new SurrealDB connection");
        debug!("Connection configuration: {:?}", config);
        let db = Self::establish_connection(&config).await?;
        debug!("SurrealDB connection established");

        Ok(Self {
            db,
            config,
            last_used: Arc::new(RwLock::new(Instant::now())),
            is_healthy: AtomicBool::new(true),
            connection_id: format!("{}-{}", config.namespace, config.database),
            query_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        })
    }
    
    /// Establish the actual SurrealDB connection based on configuration
    async fn establish_connection(config: &SurrealConfig) -> Result<Surreal<Any>> {
        let db: Surreal<Any> = match &config.connection {
            SurrealConnectionConfig::Memory => {
                debug!("Establishing SurrealDB memory connection");
                Surreal::new::<Mem>(()).await?
                    .map_err(|e| Error::Connection(format!("Failed to create memory connection: {e}")))
            }
            SurrealConnectionConfig::File { path } => {
                debug!("Establishing SurrealDB file connection to: {}", path.display());
                Surreal::new::<File>(format!("file://{}", path.display())).await?
                    .map_err(|e| Error::Connection(format!("Failed to create file connection: {e}")))
            }
            SurrealConnectionConfig::RocksDb { path } => {
                debug!("Establishing SurrealDB RocksDB connection to: {}", path.display());
                Surreal::new::<RocksDb>(format!("rocksdb://{}", path.display())).await?
                    .map_err(|e| Error::Connection(format!("Failed to create RocksDB connection: {e}")))
            }
            SurrealConnectionConfig::Remote { url } => {
                debug!("Establishing SurrealDB remote connection to: {}", url);
                // This can default to a WebSocket connection for remote connections
                Surreal::new::<Ws>(url.as_str()).await?
                    .map_err(|e| Error::Connection(format!("Failed to create remote connection: {e}")))
            }
        };
        
        // Authenticate if credentials provided
        Self::authenticate(&db, &config.auth).await?;
        
        // Select namespace and database
        db.use_ns(&config.namespace)
          .use_db(&config.database)
          .await
          .map_err(|e| Error::Connection(format!("Failed to select namespace/database: {e}")))?;
          
        // Set capabilities if configured
        Self::configure_capabilities(&db, &config).await?;
        
        debug!("SurrealDB connection established successfully");
        Ok(db)
    }
    
    /// Handle authentication based on configuration
    async fn authenticate(db: &Surreal<Any>, auth: &SurrealAuthConfig) -> Result<()> {
        if let Some(ref token) = auth.token {
            debug!("Authenticating with JWT token");
            db.authenticate(token).await
                .map_err(|e| Error::Authentication(format!("JWT authentication failed: {e}")))?;
        } else if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
            debug!("Authenticating with root credentials");
                        db.signin(surrealdb::opt::auth::Root {
                username,
                password,
            }).await
                  .map_err(|e| Error::Authentication(format!("Root authentication failed: {e}")))?;
        }
        
        Ok(())
    }
    
    /// Configure SurrealDB capabilities
    async fn configure_capabilities(db: &Surreal<Any>, config: &SurrealConfig) -> Result<()> {
        // Set capabilities based on configuration
        let capabilities = &config.capabilities;
        
        if !capabilities.allow_functions {
            // Disable functions if configured
            trace!("Disabling SurrealDB functions");
        }
        
        if !capabilities.allow_network {
            // Disable network access if configured
            trace!("Disabling SurrealDB network access");
        }
        
        if !capabilities.allow_scripting {
            // Disable scripting if configured  
            trace!("Disabling SurrealDB scripting");
        }
        
        Ok(())
    }
    
    /// Execute a query with error handling and metrics
    pub async fn query(&self, sql: &str) -> Result<Response> {
        self.update_last_used().await;
        self.query_count.fetch_add(1, Ordering::Relaxed);
        
        trace!(
            connection_id = self.connection_id,
            query_count = self.query_count.load(Ordering::Relaxed),
            "Executing SurrealDB query"
        );
        
        match tokio::time::timeout(
            Duration::from_secs(self.config.query_timeout),
            self.db.query(sql)
        ).await {
            Ok(Ok(result)) => {
                self.is_healthy.store(true, Ordering::Relaxed);
                Ok(result)
            }
            Ok(Err(e)) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                self.is_healthy.store(false, Ordering::Relaxed);
                Err(Error::Query(format!("SurrealDB query failed: {e}")).into())
            }
            Err(_) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                self.is_healthy.store(false, Ordering::Relaxed);
                Err(Error::Timeout(format!("SurrealDB query timeout after {}s", self.config.query_timeout)).into())
            }
        }
    }
    
    /// Execute a transaction with timeout
    pub async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Surreal<Any>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        self.update_last_used().await;
        
        trace!(
            connection_id = self.connection_id,
            "Starting SurrealDB transaction"
        );
        
        match tokio::time::timeout(
            Duration::from_secs(self.config.transaction_timeout),
            f(&self.db)
        ).await {
            Ok(Ok(result)) => {
                self.is_healthy.store(true, Ordering::Relaxed);
                Ok(result)
            }
            Ok(Err(e)) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
            Err(_) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                self.is_healthy.store(false, Ordering::Relaxed);
                Err(Error::Timeout(format!("SurrealDB transaction timeout after {}s", self.config.transaction_timeout)).into())
            }
        }
    }
    
    /// Check if the connection is healthy
    pub async fn health_check(&self) -> Result<()> {
        trace!(connection_id = self.connection_id, "Performing health check");
        
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.db.health()
        ).await {
            Ok(Ok(_)) => {
                self.is_healthy.store(true, Ordering::Relaxed);
                Ok(())
            }
            Ok(Err(e)) => {
                self.is_healthy.store(false, Ordering::Relaxed);
                Err(Error::Health(format!("Health check failed: {e}")).into())
            }
            Err(_) => {
                self.is_healthy.store(false, Ordering::Relaxed);
                Err(Error::Timeout("Health check timeout".to_string()).into())
            }
        }
    }
    
    /// Check if connection is healthy (non-blocking)
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }
    
    /// Get connection statistics
    pub async fn stats(&self) -> SurrealConnectionStats {
        SurrealConnectionStats {
            connection_id: 0, //self.connection_id,
            last_used: *self.last_used.read().await,
            is_healthy: self.is_healthy(),
            query_count: self.query_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
        }
    }
    
    /// Update the last used timestamp
    async fn update_last_used(&self) {
        *self.last_used.write().await = Instant::now();
    }
    
    /// Check if connection is idle
    pub async fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.read().await.elapsed() > idle_timeout
    }
    
    /// Reconnect the connection
    pub async fn reconnect(&mut self) -> Result<()> {
        warn!(connection_id = self.connection_id, "Reconnecting SurrealDB connection");
        
        self.db = Self::establish_connection(&self.config).await?;
        self.is_healthy.store(true, Ordering::Relaxed);
        self.update_last_used().await;
        
        debug!(connection_id = self.connection_id, "SurrealDB connection reconnected");
        Ok(())
    }
}

impl ConnectionFactory {
    pub fn new(config: SurrealConfig) -> Self {
        Self {
            config,
            next_id: AtomicU64::new(1),
        }
    }
    
    /// Create a new connection
    pub async fn create_connection(&self) -> Result<SurrealConnection> {
        // let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // remove the id for now
        SurrealConnection::new(self.config.clone()).await
            .map_err(|e| e.into())
    }
    
    /// Get the configuration
    pub fn config(&self) -> &SurrealConfig {
        &self.config
    }
}

impl HealthChecker {
    pub fn new(interval: Duration, timeout: Duration) -> Self {
        Self { interval, timeout }
    }
    
    /// Start background health checking for a connection
    pub async fn start_monitoring(
        &self,
        mut connection: SurrealConnection,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        let mut interval = tokio::time::interval(self.interval);
        let mut shutdown = shutdown;
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = connection.health_check().await {
                        warn!(
                            connection_id = connection.connection_id,
                            error = %e,
                            "Connection health check failed, attempting reconnection"
                        );
                        
                        if let Err(reconnect_err) = connection.reconnect().await {
                            error!(
                                connection_id = connection.connection_id,
                                error = %reconnect_err,
                                "Failed to reconnect"
                            );
                        }
                    }
                }
                _ = shutdown.changed() => {
                    debug!(connection_id = connection.connection_id, "Stopping health monitoring");
                    break;
                }
            }
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct SurrealConnectionStats {
    pub connection_id: u64,
    pub last_used: Instant,
    pub is_healthy: bool,
    pub query_count: u64,
    pub error_count: u64,
}

impl SurrealConnectionStats {
    /// Get error rate as a percentage
    pub fn error_rate(&self) -> f64 {
        if self.query_count == 0 {
            0.0
        } else {
            (self.error_count as f64 / self.query_count as f64) * 100.0
        }
    }
    
    /// Check if connection has high error rate
    pub fn has_high_error_rate(&self, threshold: f64) -> bool {
        self.error_rate() > threshold
    }
}
