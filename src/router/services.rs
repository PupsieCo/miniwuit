use crate::{Guard, State, state};
use axum::Router;
use conduwuit::Server;
use service::ServicesTrait;
use std::sync::Arc;

/// Router-specific service collections
pub trait RouterServices: ServicesTrait {
	type Guard: Send + Sync;

	fn arc_build(&self) -> (Router, Self::Guard) {
		panic!("arc_build should never be called on a non-Arc or non-Arc-tuple")
	}

	fn build(router: Router<State<Self>>, server: &Server) -> Router<State<Self>>;
}

impl<T: RouterServices> RouterServices for Arc<T> {
	type Guard = Guard<T>;

	fn arc_build(&self) -> (Router, Self::Guard) {
		let router = Router::<State<T>>::new();
		let (state, guard) = state::create(self.clone());
		let router = T::build(router, &self.server()).with_state(state);

		(router, guard)
	}

	fn build(_router: Router<State<Self>>, _server: &Server) -> Router<State<Self>> {
		panic!("build should never be called on an Arc")
	}
}

// TODO: use macro to impl this for arbitrary sized tuples
impl<T1: RouterServices, T2: RouterServices> RouterServices for (T1, T2) {
	type Guard = (T1::Guard, T2::Guard);

	fn arc_build(&self) -> (Router, Self::Guard) {
		let (router1, guard1) = self.0.arc_build();
		let (router2, guard2) = self.1.arc_build();
		(router1.merge(router2), (guard1, guard2))
	}

	fn build(_router: Router<State<Self>>, _server: &Server) -> Router<State<Self>> {
		panic!("build should never be called on a tuple")
	}
}
