//! The server's public types. Consumers select `default-features = false,
//! features = ["contract"]`, which builds without a runtime dependency.
use veoveo_hub_sdk::calls::ProviderContract;
use veoveo_types::ServerSlug;

mod logs;
mod permissions;
mod repository;
mod selection;
mod status;

pub use logs::{
    DEFAULT_LOG_LINES, LogLine, LogSource, Logs, LogsRequest, MAX_LOG_LINE_BYTES, MAX_LOG_LINES,
};
pub use permissions::{ConsolePermission, ConsoleScope};
pub use repository::{CreateRepositoryRequest, Repository};
pub use selection::{SelectRequest, Selection, WatchRequest, Watching};
pub use status::{Condition, Failure, InstanceStatus, Phase, SandboxStatus, Stage, StatusRequest};

/// The console as the provider of its tools.
pub struct ConsoleProvider;

impl ProviderContract for ConsoleProvider {
    fn slug() -> ServerSlug {
        crate::server()
    }
}
