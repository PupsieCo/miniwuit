use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use conduwuit::{Result, Server, debug, info, warn};
use database::{Deserialized, Map};
use ruma::events::room::message::RoomMessageEventContent;
use serde::Deserialize;
use tokio::{
	sync::Notify,
	time::{MissedTickBehavior, interval},
};

use crate::{Dep, admin, client, globals};

pub struct Service {
	interval: Duration,
	interrupt: Notify,
	db: Arc<Map>,
	services: Services,
}

struct Services {
	admin: Dep<admin::Service>,
	client: Dep<client::Service>,
	globals: Dep<globals::Service>,
	server: Arc<Server>,
}

#[derive(Debug, Deserialize)]
struct CheckForUpdatesResponse {
	updates: Vec<CheckForUpdatesResponseEntry>,
}

#[derive(Debug, Deserialize)]
struct CheckForUpdatesResponseEntry {
	id: u64,
	date: String,
	message: String,
}

const CHECK_FOR_UPDATES_URL: &str = "https://pupbrain.dev/check-for-updates/stable";
const CHECK_FOR_UPDATES_INTERVAL: u64 = 7200; // 2 hours
const LAST_CHECK_FOR_UPDATES_COUNT: &[u8; 1] = b"u";

#[async_trait]
impl crate::Service for Service {
	fn build(args: crate::Args<'_>) -> Result<Arc<Self>> {
		Ok(Arc::new(Self {
			interval: Duration::from_secs(CHECK_FOR_UPDATES_INTERVAL),
			interrupt: Notify::new(),
			db: args.db["global"].clone(),
			services: Services {
				globals: args.depend::<globals::Service>("globals"),
				admin: args.depend::<admin::Service>("admin"),
				client: args.depend::<client::Service>("client"),
				server: args.server.clone(),
			},
		}))
	}

	#[tracing::instrument(skip_all, name = "updates", level = "debug")]
	async fn worker(self: Arc<Self>) -> Result<()> {
		if !self.services.globals.allow_check_for_updates() {
			debug!("Disabling update check");
			return Ok(());
		}

		let mut i = interval(self.interval);
		i.set_missed_tick_behavior(MissedTickBehavior::Delay);
		i.reset_after(self.interval);
		loop {
			tokio::select! {
				() = self.interrupt.notified() => break,
				_ = i.tick() => (),
			}

			if let Err(e) = self.check().await {
				warn!(%e, "Failed to check for updates");
			}
		}

		Ok(())
	}

	fn interrupt(&self) { self.interrupt.notify_waiters(); }

	fn name(&self) -> &str { crate::service::make_name(std::module_path!()) }
}

impl Service {
	#[tracing::instrument(skip_all)]
	async fn check(&self) -> Result<()> {
		debug_assert!(self.services.server.running(), "server must not be shutting down");

		let response = self
			.services
			.client
			.default
			.get(CHECK_FOR_UPDATES_URL)
			.send()
			.await?
			.text()
			.await?;

		let response = serde_json::from_str::<CheckForUpdatesResponse>(&response)?;
		for update in &response.updates {
			if update.id > self.last_check_for_updates_id().await {
				self.handle(update).await;
				self.update_check_for_updates_id(update.id);
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn handle(&self, update: &CheckForUpdatesResponseEntry) {
		info!("{} {:#}", update.date, update.message);
		self.services
			.admin
			.send_message(RoomMessageEventContent::text_markdown(format!(
				"### the following is a message from the conduwuit puppy\n\nit was sent on \
				 `{}`:\n\n@room: {}",
				update.date, update.message
			)))
			.await
			.ok();
	}

	#[inline]
	pub fn update_check_for_updates_id(&self, id: u64) {
		self.db.raw_put(LAST_CHECK_FOR_UPDATES_COUNT, id);
	}

	pub async fn last_check_for_updates_id(&self) -> u64 {
		self.db
			.get(LAST_CHECK_FOR_UPDATES_COUNT)
			.await
			.deserialized()
			.unwrap_or(0_u64)
	}
}
