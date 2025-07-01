use crate::{
	config,
	service::{Args, Map, Service},
};
use async_trait::async_trait;
use conduwuit::{Result, Server, debug, debug_info, info, trace, utils::stream::IterStream};
use database::Database;
use futures::{Stream, StreamExt, TryStreamExt};
use service::Manager;
use service::services::ServicesTrait;
use std::{
	any::Any,
	collections::BTreeMap,
	sync::{Arc, RwLock},
};
use tokio::sync::Mutex;
use conduwuit_router::{Guard, Router, RouterServices, State};

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
		// FIXME: We're currently creating a new db for each set of services
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

	fn db(&self) -> Arc<Database> {
		self.db.clone()
	}

	fn name(&self) -> String {
		"CoreServices".to_string()
	}

	fn check_refs(self) {}

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

impl RouterServices for Services {
	type Guard = Guard<Services>;

	fn build(router: Router<State<Self>>, _server: &Server) -> Router<State<Self>> {
		router
	}
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
