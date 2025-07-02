use axum::Router;
use service::ServicesTrait;

/// Router-specific service collections
pub trait RouterServices: 'static {
	type Services: ServicesTrait + Clone;
	type Guard: Send + Sync;

	fn build(services: Self::Services) -> (Router, Self::Guard);
}

// TODO: use macro to impl this for arbitrary sized tuples
impl<T1: RouterServices, T2: RouterServices> RouterServices for (T1, T2) {
	type Services = (T1::Services, T2::Services);
	type Guard = (T1::Guard, T2::Guard);

	fn build(services: Self::Services) -> (Router, Self::Guard) {
		let (router1, guard1) = T1::build(services.0);
		let (router2, guard2) = T2::build(services.1);
		(router1.merge(router2), (guard1, guard2))
	}
}
