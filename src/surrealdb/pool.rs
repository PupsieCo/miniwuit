use std::sync::Arc;
use async_channel::{Receiver, Sender};
use surrealdb::{Surreal, engine::any::Any};
use tokio::sync::Semaphore;
use conduwuit::Result;

use crate::{
    config::{SurrealConfig, SurrealPoolConfig},
    error::Error,
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
}

impl ConnectionPool {
    pub async fn new(config: &SurrealConfig) -> Result<Arc<Self>> {
        let (sender, receiver) = async_channel::bounded(config.pool.max_connections);
        
        let pool = Arc::new(Self {
            connections: Arc::new(Semaphore::new(config.pool.max_connections)),
            sender,
            receiver,
            config: config.pool.clone(),
        }).await?;
        
        // Pre-fill pool with connections
        pool.initialize_connections(config).await?;
        
        Ok(pool)
    }
    
    pub async fn get_connection(&self) -> Result<PooledConnection> {
        let _permit = self.connections.acquire().await?;
        
        match self.receiver.try_recv() {
            Ok(mut conn) => {
                // Validate connection is still alive
                if conn.is_healthy().await {
                    conn.last_used = std::time::Instant::now();
                    Ok(conn)
                } else {
                    // Create new connection
                    self.create_connection().await
                }
            }
            Err(_) => {
                // No available connections, create new one
                self.create_connection().await
            }
        }
    }
    
    pub async fn return_connection(&self, conn: PooledConnection) -> Result<()> {
        self.sender.send(conn).await?;
        Ok(())
    }

    async fn initialize_connections(&self, config: &SurrealConfig) -> Result<()> {
        for _ in 0..config.pool.max_connections {
            let conn = self.create_connection().await?;
            self.sender.send(conn).await?;
        }
        Ok(())
    }
}