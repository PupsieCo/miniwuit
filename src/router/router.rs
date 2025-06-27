use std::sync::Arc;

use axum::{Router, response::IntoResponse, routing::get};
use conduwuit::{Error, Server};
// use conduwuit_social_api::router::{state, state::Guard};
use crate::{state, state::Guard};
use conduwuit_service::{Services, ServicesTrait};
use http::{StatusCode, Uri};
use ruma::api::client::error::ErrorKind;

pub(crate) fn build<R: SubRouter>(services: &R::Services) -> (Router, R::Guard) {
	// let router = Router::<state::State>::new();
	// let (state, guard) = state::create(services.clone());
	// let router = conduwuit_social_api::router::build(router, &services.server)
	// 	.route("/", get(it_works))
	// 	.fallback(not_found)
	// 	.with_state(state);
	// 
	// (router, guard)
	R::build(&services.server())
}

async fn not_found(_uri: Uri) -> impl IntoResponse {
	Error::Request(ErrorKind::Unrecognized, "Not Found".into(), StatusCode::NOT_FOUND)
}

async fn it_works() -> &'static str {
	"hewwo from conduwuit woof!"
}

pub trait SubRouter {
	type Services: ServicesTrait;
	type Guard;

	fn build(server: &Server) -> (Router, Self::Guard);
}

// TODO: use macro to impl this for arbitrary sized tuples
impl<T1: SubRouter, T2: SubRouter> SubRouter for (T1, T2) {
	type Services = (T1::Services, T2::Services);
	type Guard = (Guard<T1::Services>, Guard<T2::Services>);

	fn build(server: &Server) -> (Router, Self::Guard) {
		let (router1, guard1) = T1::build(server);
		let (router2, guard2) = T2::build(server);
		(router1.merge(router2), (guard1, guard2))
	}
}
