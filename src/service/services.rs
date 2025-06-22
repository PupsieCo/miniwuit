use std::{
	any::Any,
	collections::BTreeMap,
	sync::{Arc, RwLock},
};

use conduwuit::{Result, Server, debug, debug_info, info, trace, utils::stream::IterStream};
use database::Database;
use futures::{Stream, StreamExt, TryStreamExt};
use tokio::sync::Mutex;

use crate::{
	account_data, admin, appservice, client, config, emergency, federation, globals, key_backups,
	manager::Manager,
	media, presence, pusher, resolver, rooms, sending, server_keys, service,
	service::{Args, Map, Service},
	sync, transaction_ids, uiaa, updates, users,
};

pub struct Services {
	pub config: Arc<config::Service>,
	manager: Mutex<Option<Arc<Manager>>>,
	pub(crate) service: Arc<Map>,
	pub server: Arc<Server>,
	pub db: Arc<Database>,
}

impl Services {
	#[allow(clippy::cognitive_complexity)]
	pub async fn build(server: Arc<Server>) -> Result<Arc<Self>> {
		let db = Database::open(&server).await?;
		let service: Arc<Map> = Arc::new(RwLock::new(BTreeMap::new()));
		macro_rules! build {
			($tyname:ty) => {{
				let built = <$tyname>::build(Args {
					db: &db,
					server: &server,
					service: &service,
				})?;
				add_service(&service, built.clone(), built.clone());
				built
			}};
		}

		Ok(Arc::new(Self {
			config: build!(config::Service),
			manager: Mutex::new(None),
			service,
			server,
			db,
		}))
	}

	pub async fn start(self: &Arc<Self>) -> Result<Arc<Self>> {
		debug_info!("Starting services...");

		self.admin.set_services(Some(Arc::clone(self)).as_ref());
		super::migrations::migrations(self).await?;
		self.manager
			.lock()
			.await
			.insert(Manager::new(self))
			.clone()
			.start()
			.await?;

		// reset dormant online/away statuses to offline, and set the server user as
		// online
		if self.server.config.allow_local_presence && !self.db.is_read_only() {
			self.presence.unset_all_presence().await;
			_ = self
				.presence
				.ping_presence(&self.globals.server_user, &ruma::presence::PresenceState::Online)
				.await;
		}

		debug_info!("Services startup complete.");
		Ok(Arc::clone(self))
	}

	pub async fn stop(&self) {
		info!("Shutting down services...");

		// set the server user as offline
		if self.server.config.allow_local_presence && !self.db.is_read_only() {
			_ = self
				.presence
				.ping_presence(&self.globals.server_user, &ruma::presence::PresenceState::Offline)
				.await;
		}

		self.interrupt();
		if let Some(manager) = self.manager.lock().await.as_ref() {
			manager.stop().await;
		}

		self.admin.set_services(None);

		debug_info!("Services shutdown complete.");
	}

	pub async fn poll(&self) -> Result<()> {
		if let Some(manager) = self.manager.lock().await.as_ref() {
			return manager.poll().await;
		}

		Ok(())
	}

	pub async fn clear_cache(&self) {
		self.services()
			.for_each(|service| async move {
				service.clear_cache().await;
			})
			.await;
	}

	pub async fn memory_usage(&self) -> Result<String> {
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
		for (name, (service, ..)) in self.service.read().expect("locked for reading").iter() {
			if let Some(service) = service.upgrade() {
				trace!("Interrupting {name}");
				service.interrupt();
			}
		}
	}

	/// Iterate from snapshot of the services map
	fn services(&self) -> impl Stream<Item = Arc<dyn Service>> + Send {
		self.service
			.read()
			.expect("locked for reading")
			.values()
			.filter_map(|val| val.0.upgrade())
			.collect::<Vec<_>>()
			.into_iter()
			.stream()
	}

	#[inline]
	pub fn try_get<T>(&self, name: &str) -> Result<Arc<T>>
	where
		T: Any + Send + Sync + Sized,
	{
		service::try_get::<T>(&self.service, name)
	}

	#[inline]
	pub fn get<T>(&self, name: &str) -> Option<Arc<T>>
	where
		T: Any + Send + Sync + Sized,
	{
		service::get::<T>(&self.service, name)
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
