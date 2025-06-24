use std::sync::Arc;
use async_channel::{Receiver, Sender};
use surrealdb::{Surreal, engine::any::Any};
use tokio::sync::Semaphore;
use conduwuit::Result;

use crate::{
    config::{SurrealConfig, SurrealPoolConfig},
    error::{Error, Result as SurrealResult},
};

pub struct ConnectionPool {
    connections: Arc<Semaphore>,
    sender: Sender<PooledConnection>,
    receiver: Receiver<PooledConnection>,
    config: SurrealPoolConfig
}

pub struct PooledConnection {
    pub db: Surreal<Any>,
    pub last_used: std::time::Instant,
    pub config: SurrealConfig,
}

impl PooledConnection {
    pub async fn is_healthy(&self) -> bool {
        // Simple health check - try a basic query
        match self.db.health().await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl ConnectionPool {
    pub async fn new(config: SurrealConfig) -> SurrealResult<Arc<Self>> {
        let (sender, receiver) = async_channel::bounded(config.pool.max_connections);
        
        let pool = Arc::new(Self {
            connections: Arc::new(Semaphore::new(config.pool.max_connections)),
            sender,
            receiver,
            config: config.pool.clone(),
        });
        
        // Pre-fill pool with connections
        pool.initialize_connections(&config).await?;
        
        Ok(pool.clone())
    }
    
    pub async fn get_connection(&self) -> SurrealResult<PooledConnection> {
        let _permit = self.connections.acquire().await
            .map_err(|e| Error::Pool(format!("Failed to acquire connection: {e}")))?;
        
        match self.receiver.try_recv() {
            Ok(mut conn) => {
                // Validate connection is still alive
                if conn.is_healthy().await {
                    conn.last_used = std::time::Instant::now();
                    Ok(conn)
                } else {
                    // Create new connection
                    self.create_new_connection().await
                }
            }
            Err(_) => {
                // No available connections, create new one
                self.create_new_connection().await
            }
        }
    }
    
    pub async fn return_connection(&self, conn: PooledConnection) -> SurrealResult<()> {
        self.sender.send(conn).await
            .map_err(|e| Error::Pool(format!("Failed to return connection: {e}")))?;
        Ok(())
    }

    async fn initialize_connections(&self, config: &SurrealConfig) -> SurrealResult<()> {
        for _ in 0..config.pool.max_connections {
            let conn = self.create_new_connection().await?;
            self.sender.send(conn).await
                .map_err(|e| Error::Pool(format!("Failed to initialize connection: {e}")))?;
        }
        Ok(())
    }

    async fn create_new_connection(&self) -> SurrealResult<PooledConnection> {
        use crate::connection::ConnectionFactory;
        
        // Create a temporary config - in real implementation this would be stored
        let factory = ConnectionFactory::new(crate::config::SurrealConfig::default());
        let connection = factory.create_connection().await?;
        
        Ok(PooledConnection {
            db: connection.db,
            last_used: std::time::Instant::now(),
        })
    }
}