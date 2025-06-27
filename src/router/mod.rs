#![type_length_limit = "32768"] //TODO: reduce me

mod layers;
mod request;
mod router;
mod run;
mod serve;
mod state;

extern crate conduwuit_core as conduwuit;

use std::{panic::AssertUnwindSafe, pin::Pin, sync::Arc};

use conduwuit::{Error, Result, Server};
use conduwuit_service::Services as CoreServices;
use conduwuit_social_service::Services as SocialServices;
use futures::{Future, FutureExt, TryFutureExt};

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}

#[unsafe(no_mangle)]
pub extern "Rust" fn start(
	server: &Arc<Server>,
) -> Pin<Box<dyn Future<Output = Result<Arc<Services>>> + Send>> {
	AssertUnwindSafe(run::start(server.clone()))
		.catch_unwind()
		.map_err(Error::from_panic)
		.unwrap_or_else(Err)
		.boxed()
}

#[unsafe(no_mangle)]
pub extern "Rust" fn stop(
	core_services: Arc<CoreServices>,
	social_services: Arc<SocialServices>,
) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
	AssertUnwindSafe(run::stop(core_services, social_services))
		.catch_unwind()
		.map_err(Error::from_panic)
		.unwrap_or_else(Err)
		.boxed()
}

#[unsafe(no_mangle)]
pub extern "Rust" fn run(
	core_services: &Arc<CoreServices>,
	social_services: &Arc<SocialServices>,
) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
	AssertUnwindSafe(run::run(core_services.clone(), social_services.clone()))
		.catch_unwind()
		.map_err(Error::from_panic)
		.unwrap_or_else(Err)
		.boxed()
}
