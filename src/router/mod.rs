#![type_length_limit = "32768"] //TODO: reduce me

mod layers;
mod request;
mod router;
mod run;
mod serve;
mod services;
pub mod state;

extern crate conduwuit_core as conduwuit;
extern crate conduwuit_service_core as service;

use std::{panic::AssertUnwindSafe, pin::Pin, sync::Arc};

use conduwuit::{Error, Result, Server};
use futures::{Future, FutureExt, TryFutureExt};

pub use axum::routing::Router;
use service::ServicesTrait;
pub use services::RouterServices;
pub use state::Guard;
pub use state::State;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}

// #[unsafe(no_mangle)]
pub fn start<S: ServicesTrait>(
	server: &Arc<Server>,
) -> Pin<Box<dyn Future<Output = Result<S::BuildResult>> + Send>> {
	AssertUnwindSafe(run::start::<S>(server.clone()))
		.catch_unwind()
		.map_err(Error::from_panic)
		.unwrap_or_else(Err)
		.boxed()
}

// #[unsafe(no_mangle)]
pub fn stop<S: ServicesTrait>(services: S) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
	AssertUnwindSafe(run::stop(services))
		.catch_unwind()
		.map_err(Error::from_panic)
		.unwrap_or_else(Err)
		.boxed()
}

// #[unsafe(no_mangle)]
pub fn run<R: RouterServices>(
	services: R::Services,
) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
	AssertUnwindSafe(run::run::<R>(services))
		.catch_unwind()
		.map_err(Error::from_panic)
		.unwrap_or_else(Err)
		.boxed()
}
