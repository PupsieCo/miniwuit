use async_trait::async_trait;
use conduwuit::{Result, Server, debug, debug_info, info, trace, utils::stream::IterStream};
use database::Database;
use futures::{Stream, StreamExt, TryStreamExt};
use std::{
	any::Any,
	collections::BTreeMap,
	sync::{Arc, RwLock},
};
use tokio::sync::Mutex;

use crate::{
	MapVal, config,
	manager::Manager,
	service::{Args, Map, Service},
};

/// Abstract interface for Services
#[async_trait]
pub trait ServicesTrait: Any + Send + Sync {
	// type Services: ServicesTrait;
	type BuildResult: Send;

	// Core methods that need to be implemented
	fn server(&self) -> Arc<Server>;
	fn service_map(&self) -> Arc<Map>;
	// fn config(&self) -> &Arc<config::Service>;
	// fn db(&self) -> &Arc<Database>;
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

	// fn try_get<T>(&self, name: &str) -> Result<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized;
	//
	// fn get<T>(&self, name: &str) -> Option<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized;
}

pub struct Services {
	pub config: Arc<config::Service>,
	manager: Mutex<Option<Arc<Manager<Self>>>>,
	pub(crate) service_map: Arc<Map>,
	pub server: Arc<Server>,
	pub db: Arc<Database>,
}

#[async_trait]
impl ServicesTrait for Services {
	// type Services = Self;
	type BuildResult = Arc<Self>;

	#[allow(clippy::cognitive_complexity)]
	async fn start(server: Arc<Server>) -> Result<Arc<Self>> {
		let db = Database::open(&server).await?;
		let service_map: Arc<Map> = Arc::new(RwLock::new(BTreeMap::new()));
		macro_rules! build {
			($tyname:ty) => {{
				let built = <$tyname>::build(Args {
					db: &db,
					server: &server,
					service: &service_map,
				})?;
				add_service(&service_map, built.clone(), built.clone());
				built
			}};
		}

		let built = Arc::new(Self {
			config: build!(config::Service),
			manager: Mutex::new(None),
			service_map,
			server,
			db,
		});

		debug_info!("Starting services...");

		// self.admin.set_services(Some(Arc::clone(self)).as_ref());
		// super::migrations::migrations(self).await?;
		built
			.manager
			.lock()
			.await
			.insert(Manager::new(&built))
			.clone()
			.start()
			.await?;

		// reset dormant online/away statuses to offline, and set the server user as
		// online
		// if self.server.config.allow_local_presence && !self.db.is_read_only() {
		// 	self.presence.unset_all_presence().await;
		// 	_ = self
		// 		.presence
		// 		.ping_presence(&self.globals.server_user, &ruma::presence::PresenceState::Online)
		// 		.await;
		// }

		debug_info!("Services startup complete.");
		Ok(built)
	}

	async fn stop(&self) {
		info!("Shutting down services...");

		// set the server user as offline
		// if self.server.config.allow_local_presence && !self.db.is_read_only() {
		// 	_ = self
		// 		.presence
		// 		.ping_presence(&self.globals.server_user, &ruma::presence::PresenceState::Offline)
		// 		.await;
		// }

		self.interrupt();
		if let Some(manager) = self.manager.lock().await.as_ref() {
			manager.stop().await;
		}

		// self.admin.set_services(None);

		debug_info!("Services shutdown complete.");
	}

	async fn poll(&self) -> Result<()> {
		if let Some(manager) = self.manager.lock().await.as_ref() {
			return manager.poll().await;
		}

		Ok(())
	}

	// #[inline]
	// fn try_get<T>(&self, name: &str) -> Result<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized,
	// {
	// 	service::try_get::<T>(&self.service_map(), name)
	// }
	//
	// #[inline]
	// fn get<T>(&self, name: &str) -> Option<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized,
	// {
	// 	service::get::<T>(&self.service_map(), name)
	// }

	fn server(&self) -> Arc<Server> {
		self.server.clone()
	}

	fn service_map(&self) -> Arc<Map> {
		self.service_map.clone()
	}

	// fn config(&self) -> &Arc<config::Service> {
	// 	&self.config
	// }
	//
	// fn db(&self) -> &Arc<Database> {
	// 	&self.db
	// }
	//
	// fn manager(&self) -> Option<&Mutex<Option<Arc<Manager<Self>>>>> {
	// 	Some(&self.manager)
	// }
}

#[allow(clippy::needless_pass_by_value)]
fn add_service(map: &Arc<Map>, s: Arc<dyn Service>, a: Arc<dyn Any + Send + Sync>) {
	let name = s.name();
	let len = map.read().expect("locked for reading").len();

	trace!("built service #{len}: {name:?}");
	map.write()
		.expect("locked for writing")
		.insert(name.to_owned(), (Arc::downgrade(&s), Arc::downgrade(&a)));
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
}

fn copy_service_map(out_map: &mut BTreeMap<String, MapVal>, in_map: &Map) {
	for (key, val) in in_map.read().expect("locked for reading").iter() {
		out_map.insert(key.clone(), val.clone());
	}
}
