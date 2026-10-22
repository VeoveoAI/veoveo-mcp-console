//! `select` and `watch`: which branch serves the caller each server, and which
//! repositories the Hub follows for the caller.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The arguments of `select`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Selection, name = "select")]
pub struct SelectRequest {
    /// The server's slug, such as `time`.
    pub server: String,
    /// The branch whose instance serves you the server, in every chat; null returns
    /// the server to the installation's base branch.
    pub branch: Option<String>,
}

/// Your selection for one server after `select`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Selection {
    pub server: String,
    /// The selected branch, or null for the base branch.
    pub branch: Option<String>,
}

/// The arguments of `watch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Watching, name = "watch")]
pub struct WatchRequest {
    /// The Lore repository's identifier.
    pub repository: String,
}

/// A repository the Hub watches for you: each branch with an executable
/// `openshell/run` that someone pushes is built and served as an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Watching {
    pub repository: String,
}
