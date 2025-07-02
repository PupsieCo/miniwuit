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
	account_data, admin, appservice, client, emergency, federation, globals, key_backups, media,
	presence, pusher, resolver, rooms, sending, server_keys, sync, transaction_ids, uiaa,
	updates, users,
};

use service_core::{Args, Manager, Map, Service, ServicesTrait};
use conduwuit_service::config;

pub struct Services {
	pub account_data: Arc<account_data::Service>,
	pub admin: Arc<admin::Service>,
	pub appservice: Arc<appservice::Service>,
	pub config: Arc<config::Service>,
	pub client: Arc<client::Service>,
	pub emergency: Arc<emergency::Service>,
	pub globals: Arc<globals::Service>,
	pub key_backups: Arc<key_backups::Service>,
	pub media: Arc<media::Service>,
	pub presence: Arc<presence::Service>,
	pub pusher: Arc<pusher::Service>,
	pub resolver: Arc<resolver::Service>,
	pub rooms: rooms::Service,
	pub federation: Arc<federation::Service>,
	pub sending: Arc<sending::Service>,
	pub server_keys: Arc<server_keys::Service>,
	pub sync: Arc<sync::Service>,
	pub transaction_ids: Arc<transaction_ids::Service>,
	pub uiaa: Arc<uiaa::Service>,
	pub updates: Arc<updates::Service>,
	pub users: Arc<users::Service>,

	manager: Mutex<Option<Arc<Manager<Self>>>>,
	pub(crate) service: Arc<Map>,
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

		let built = Arc::new(Self {
			account_data: build!(account_data::Service),
			admin: build!(admin::Service),
			appservice: build!(appservice::Service),
			resolver: build!(resolver::Service),
			client: build!(client::Service),
			config: build!(config::Service),
			emergency: build!(emergency::Service),
			globals: build!(globals::Service),
			key_backups: build!(key_backups::Service),
			media: build!(media::Service),
			presence: build!(presence::Service),
			pusher: build!(pusher::Service),
			rooms: rooms::Service {
				alias: build!(rooms::alias::Service),
				auth_chain: build!(rooms::auth_chain::Service),
				directory: build!(rooms::directory::Service),
				event_handler: build!(rooms::event_handler::Service),
				lazy_loading: build!(rooms::lazy_loading::Service),
				metadata: build!(rooms::metadata::Service),
				outlier: build!(rooms::outlier::Service),
				pdu_metadata: build!(rooms::pdu_metadata::Service),
				read_receipt: build!(rooms::read_receipt::Service),
				search: build!(rooms::search::Service),
				short: build!(rooms::short::Service),
				spaces: build!(rooms::spaces::Service),
				state: build!(rooms::state::Service),
				state_accessor: build!(rooms::state_accessor::Service),
				state_cache: build!(rooms::state_cache::Service),
				state_compressor: build!(rooms::state_compressor::Service),
				threads: build!(rooms::threads::Service),
				timeline: build!(rooms::timeline::Service),
				typing: build!(rooms::typing::Service),
				user: build!(rooms::user::Service),
			},
			federation: build!(federation::Service),
			sending: build!(sending::Service),
			server_keys: build!(server_keys::Service),
			sync: build!(sync::Service),
			transaction_ids: build!(transaction_ids::Service),
			uiaa: build!(uiaa::Service),
			updates: build!(updates::Service),
			users: build!(users::Service),

			manager: Mutex::new(None),
			service,
			server,
			db,
		});

		debug_info!("Starting services...");

		built.admin.set_services(Some(Arc::clone(&built)).as_ref());
		super::migrations::migrations(&built).await?;
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
		if built.server.config.allow_local_presence && !built.db.is_read_only() {
			built.presence.unset_all_presence().await;
			_ = built
				.presence
				.ping_presence(&built.globals.server_user, &ruma::presence::PresenceState::Online)
				.await;
		}

		debug_info!("Services startup complete.");
		Ok(built)
	}

	async fn stop(&self) {
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

	async fn poll(&self) -> Result<()> {
		if let Some(manager) = self.manager.lock().await.as_ref() {
			return manager.poll().await;
		}

		Ok(())
	}

	// /// Iterate from snapshot of the services map
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

	// #[inline]
	// fn try_get<T>(&self, name: &str) -> Result<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized,
	// {
	// 	conduwuit_service::service::try_get::<T>(&self.service, name)
	// }
	//
	// #[inline]
	// fn get<T>(&self, name: &str) -> Option<Arc<T>>
	// where
	// 	T: Any + Send + Sync + Sized,
	// {
	// 	conduwuit_service::service::get::<T>(&self.service, name)
	// }

	fn server(&self) -> Arc<Server> {
		self.server.clone()
	}

	fn service_map(&self) -> Arc<Map> {
		self.service.clone()
	}

	fn db(&self) -> Arc<Database> {
		self.db.clone()
	}

	fn name(&self) -> String {
		"SocialServices".to_string()
	}

	fn check_refs(self) {}

	//
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
