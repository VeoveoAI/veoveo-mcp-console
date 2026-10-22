//! The server's public types. Consumers select `default-features = false,
//! features = ["contract"]`, which builds without a runtime dependency.
use veoveo_hub_sdk::calls::ProviderContract;
use veoveo_types::ServerSlug;

mod permissions;

pub use permissions::{ConsolePermission, ConsoleScope};

/// The console as the provider of its tools.
pub struct ConsoleProvider;

impl ProviderContract for ConsoleProvider {
    fn slug() -> ServerSlug {
        crate::server()
    }
}
