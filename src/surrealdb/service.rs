use std::sync::Arc;
use async_trait::async_trait;
use conduwuit::{Result, Server, error, debug};
use tokio::sync::OnceCell;

use crate::{
    engine::Engine,
    context::Context,
};

pub struct Service {
    server: Arc<Server>,
    engine: OnceCell<Arc<Engine>>,
}

#[async_trait]
impl conduwuit::Service for Service {
    fn build(args: conduwuit::Args<'_>) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            server: args.server.clone(),
            engine: OnceCell::new(),
        }))
    }
    
    async fn worker(self: Arc<Self>) -> Result {
        // Initialize the engine on first run
        let engine = self.engine.get_or_try_init(|| async {
            let ctx = Context::new(&self.server)?;
            Engine::open(ctx).await
        }).await?;
        
        debug!("SurrealDB service worker started");
        
        // Background maintenance tasks
        while self.server.running() {
            // Health checks
            if let Err(e) = engine.ping().await {
                error!("SurrealDB health check failed: {e}");
            }
            
            // Wait before next maintenance cycle
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
        
        debug!("SurrealDB service worker stopped");
        Ok(())
    }
    
    fn name(&self) -> &str {
        conduwuit::service::make_name(std::module_path!())
    }
}