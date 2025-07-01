use axum::{Router, response::IntoResponse};
use conduwuit::Error;
// use conduwuit_social_api::router::{state, state::Guard};
use crate::RouterServices;
use http::{StatusCode, Uri};
use ruma::api::client::error::ErrorKind;

pub(crate) fn build<R: RouterServices>(services: &R) -> (Router, R::Guard) {
	// let router = Router::<state::State>::new();
	// let (state, guard) = state::create(services.clone());
	// let router = conduwuit_social_api::router::build(router, &services.server)
	// 	.route("/", get(it_works))
	// 	.fallback(not_found)
	// 	.with_state(state);
	// 
	// (router, guard)
	services.arc_build()
}
