use std::sync::Arc;
use async_trait::async_trait;
use conduwuit::{Result, Server, debug, info, warn};
use tokio::sync::OnceCell;

use crate::{
    engine::Engine,
    context::Context,
};

pub struct Service {
    server: Arc<Server>,
    engine: OnceCell<Arc<Engine>>,
}

impl Service {
    pub fn new(server: Arc<Server>) -> Arc<Self> {
        Arc::new(Self {
            server,
            engine: OnceCell::new(),
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("Starting SurrealDB service");
        
        // Initialize the engine
        let ctx = Context::new(&self.server).await?;
        let engine = Engine::open(ctx).await?;
        
        if let Err(_) = self.engine.set(engine) {
            warn!("SurrealDB engine was already initialized");
        }
        
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping SurrealDB service");
        Ok(())
    }
    
    pub async fn ready(&self) -> Result<()> {
        debug!("SurrealDB service ready check");
        if let Some(engine) = self.engine.get() {
            engine.ping().await?;
        }
        Ok(())
    }
    
    pub fn name(&self) -> &str {
        "surrealdb"
    }
    
    pub fn get_engine(&self) -> Option<&Arc<Engine>> {
        self.engine.get()
    }
    
    async fn worker(self: Arc<Self>) -> Result<()> {
        debug!("SurrealDB service worker started");
        
        // Background maintenance tasks
        while self.server.running() {
            if let Some(engine) = self.engine.get() {
                // Health checks
                if let Err(e) = engine.ping().await {
                    warn!("SurrealDB health check failed: {e}");
                }
            }
            
            // Wait before next maintenance cycle
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
        
        debug!("SurrealDB service worker stopped");
        Ok(())
    }
}