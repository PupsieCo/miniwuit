use crate::{Map, MapVal, Service};
use async_trait::async_trait;
use conduwuit::utils::IterStream;
use conduwuit::{Result, debug_error};
use conduwuit::{Server, debug, trace};
use database::Database;
use futures::{Stream, StreamExt, TryStreamExt};
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Abstract interface for Services
#[async_trait]
pub trait ServicesTrait: Any + Send + Sync + Sized {
	// type Services: ServicesTrait;
	type BuildResult: Send + Sync;

	// Core methods that need to be implemented
	fn server(&self) -> Arc<Server>;
	fn service_map(&self) -> Arc<Map>;
	// fn config(&self) -> &Arc<config::Service>;
	fn db(&self) -> Arc<Database>;
	// fn manager(&self) -> Option<&Mutex<Option<Arc<Manager<Self>>>>> where Self: Sized { None }

	// Methods with default implementations
	async fn clear_cache(&self) {
		self.services()
			.for_each(|service| async move {
				service.clear_cache().await;
			})
			.await;
	}

	async fn memory_usage(&self) -> Result<String> {
		self.services()
			.map(Ok)
			.try_fold(String::new(), |mut out, service| async move {
				service.memory_usage(&mut out).await?;
				Ok(out)
			})
			.await
	}

	fn interrupt(&self) {
		debug!("Interrupting services...");
		for (name, (service, ..)) in self
			.service_map()
			.read()
			.expect("locked for reading")
			.iter()
		{
			if let Some(service) = service.upgrade() {
				trace!("Interrupting {name}");
				service.interrupt();
			}
		}
	}

	// Iterate from snapshot of the services map
	fn services(&self) -> impl Stream<Item = Arc<dyn Service>> + Send {
		self.service_map()
			.read()
			.expect("locked for reading")
			.values()
			.filter_map(|val| val.0.upgrade())
			.collect::<Vec<_>>()
			.into_iter()
			.stream()
	}

	// Abstract methods that need to be implemented
	async fn start(server: Arc<Server>) -> Result<Self::BuildResult>
	where
		Self: Sized;
	async fn stop(&self);
	async fn poll(&self) -> Result<()>;

	fn name(&self) -> String;

	// fn try_get<T>(&self, name: &str) -> Result<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized;
	//
	// fn get<T>(&self, name: &str) -> Option<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized;

	fn check_refs(self);
}

#[async_trait]
impl<T: ServicesTrait + 'static> ServicesTrait for Arc<T> {
	type BuildResult = Arc<T::BuildResult>;

	fn server(&self) -> Arc<Server> {
		Arc::as_ref(self).server()
	}

	fn service_map(&self) -> Arc<Map> {
		Arc::as_ref(self).service_map()
	}

	fn db(&self) -> Arc<Database> {
		Arc::as_ref(self).db()
	}

	async fn start(server: Arc<Server>) -> Result<Self::BuildResult>
	where
		Self: Sized,
	{
		Ok(Arc::new(T::start(server).await?))
	}

	async fn stop(&self) {
		Arc::as_ref(self).stop().await
	}

	async fn poll(&self) -> Result<()> {
		Arc::as_ref(self).poll().await
	}

	fn name(&self) -> String {
		Arc::as_ref(self).name()
	}

	fn check_refs(self) {
		if let Err(arc) = Arc::try_unwrap(self) {
			debug_error!(
				"{} dangling references to {} after shutdown",
				Arc::strong_count(&arc),
				Arc::as_ref(&arc).name()
			);
		}
	}
}

// TODO: use macro to impl this for arbitrary sized tuples
#[async_trait]
impl<T1: ServicesTrait, T2: ServicesTrait> ServicesTrait for (T1, T2) {
	type BuildResult = (T1::BuildResult, T2::BuildResult);

	fn server(&self) -> Arc<Server> {
		self.0.server()
	}

	fn service_map(&self) -> Arc<Map> {
		let mut map = BTreeMap::new();
		copy_service_map(&mut map, &self.0.service_map());
		copy_service_map(&mut map, &self.1.service_map());
		Arc::new(RwLock::new(map))
	}

	fn db(&self) -> Arc<Database> {
		// FIXME: should we be doing this for sub-services?
		self.0.db()
	}

	async fn start(server: Arc<Server>) -> Result<(T1::BuildResult, T2::BuildResult)>
	where
		Self: Sized,
	{
		Ok((T1::start(server.clone()).await?, T2::start(server.clone()).await?))
	}

	async fn stop(&self) {
		self.0.stop().await;
		self.1.stop().await;
	}

	async fn poll(&self) -> Result<()> {
		self.0.poll().await?;
		self.1.poll().await
	}

	fn name(&self) -> String {
		format!("({}, {})", self.0.name(), self.1.name())
	}

	fn check_refs(self) {
		self.0.check_refs();
		self.1.check_refs();
	}
}

fn copy_service_map(out_map: &mut BTreeMap<String, MapVal>, in_map: &Map) {
	for (key, val) in in_map.read().expect("locked for reading").iter() {
		out_map.insert(key.clone(), val.clone());
	}
}
