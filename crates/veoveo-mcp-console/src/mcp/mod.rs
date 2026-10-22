//! One typed route table binds the tools, their outputs, discovery and permissions.
mod declarations;
mod requirements;
mod setup;

use std::sync::LazyLock;

use crate::contract::ConsolePermission;
use veoveo_hub_sdk::{
    hosting::{DomainServer, RoutePolicy, Routes},
    server_contract::McpServerSetup,
};

pub use declarations::{REGISTRATION, registration};
pub use requirements::AUTHORIZATION;
pub use setup::ConsoleContract;
use setup::SERVER_SETUP;

pub fn force_setup() {
    LazyLock::force(&SERVER_SETUP);
}

fn policy() -> RoutePolicy {
    RoutePolicy::permission(ConsolePermission::Use).expect("the console permission")
}

#[derive(Clone)]
pub struct ConsoleMcp;

static ROUTES: LazyLock<Routes<ConsoleMcp>> =
    LazyLock::new(|| Routes::<ConsoleMcp>::builder(policy()).build());

impl DomainServer for ConsoleMcp {
    type Contract = ConsoleContract;
    fn setup() -> &'static McpServerSetup<ConsoleContract> {
        &SERVER_SETUP
    }
    fn routes() -> &'static Routes<Self> {
        &ROUTES
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_setup_and_documents_admit() {
        std::sync::LazyLock::force(&super::SERVER_SETUP);
        assert!(super::setup::SERVER_DOCS.agent_manual().is_some());
    }
}
