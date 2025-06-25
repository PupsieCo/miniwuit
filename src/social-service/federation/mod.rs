mod execute;

use std::sync::Arc;

use conduwuit::{Result, Server};

use crate::{client, resolver, server_keys};
use conduwuit_service::{Dep, Args, Service as ServiceTrait};

pub struct Service {
	services: Services,
}

struct Services {
	server: Arc<Server>,
	client: Dep<client::Service>,
	resolver: Dep<resolver::Service>,
	server_keys: Dep<server_keys::Service>,
}

impl ServiceTrait for Service {
	fn build(args: Args<'_>) -> Result<Arc<Self>> {
		Ok(Arc::new(Self {
			services: Services {
				server: args.server.clone(),
				client: args.depend::<client::Service>("client"),
				resolver: args.depend::<resolver::Service>("resolver"),
				server_keys: args.depend::<server_keys::Service>("server_keys"),
			},
		}))
	}

	fn name(&self) -> &str { conduwuit_service::service::make_name(std::module_path!()) }
}
