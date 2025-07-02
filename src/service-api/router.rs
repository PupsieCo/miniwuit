use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;
use conduwuit::Error;
use conduwuit_router::{Guard, RouterServices, State, state};
use conduwuit_service::Services;
use http::{StatusCode, Uri};
use ruma::api::client::error::ErrorKind;
use std::sync::Arc;

pub struct ServiceApiRouter;

impl RouterServices for ServiceApiRouter {
	type Services = Arc<Services>;
	type Guard = Guard<Services>;

	fn build(services: Self::Services) -> (Router, Self::Guard) {
		let router = Router::<State<Services>>::new();
		let (state, guard) = state::create(services);
		let router = router
			.route("/", get(it_works))
			.fallback(not_found)
			.with_state(state);
		(router, guard)
	}
}

async fn not_found(_uri: Uri) -> impl IntoResponse {
	Error::Request(ErrorKind::Unrecognized, "Not Found".into(), StatusCode::NOT_FOUND)
}

async fn it_works() -> &'static str {
	"hewwo from conduwuit woof!"
}
