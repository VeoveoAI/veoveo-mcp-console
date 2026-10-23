//! `computer`: the caller's Computer, as the Hub answers `GET /computer/status`, and its
//! restart, `POST /computer/restart`. The types follow the Hub's
//! answer; fields the Hub adds later are not reported until they are added here.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The arguments of `computer`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = ComputerStatus, name = "computer")]
pub struct ComputerRequest {
    /// Restart your Computer at the current version, only when the person asks: it
    /// keeps the home `~` and ends running programs, this session's included, and
    /// empties `/tmp`. The answer comes before the restart begins.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restart: bool,
}

/// Your Computer: the version it runs and the current one, whether an update is
/// available and what it brings, a restart that runs or the last one's failure, and an
/// update installed since you were last told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct ComputerStatus {
    /// The Computer's identifier.
    pub computer: String,
    /// The version its sandbox was last created at.
    pub version: ComputerVersion,
    /// Its sandbox's phase, such as `ready`, or `none`.
    pub sandbox: String,
    /// The installation's current version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<ComputerVersion>,
    /// Why no Computer can start now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_unavailable: Option<String>,
    /// Whether the current version differs from the one the Computer runs.
    pub update_available: bool,
    /// What the current version brings over the one the Computer runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whats_new: Option<WhatsNew>,
    /// A restart that runs now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<Restart>,
    /// The last restart's failure; the Computer keeps the version it had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<RestartFailure>,
    /// Why the Hub can no longer serve the Computer safely; a restart creates it again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
    /// An update installed since you were last told, told once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<Updated>,
}

/// A Computer version: a short digest of what a sandbox gets from the installation, and
/// the first line of its release note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComputerVersion {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// What a version brings over another: its release note, a new image and the network
/// endpoints its rules gained or lost.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhatsNew {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network_rules_added: Vec<NetworkRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network_rules_removed: Vec<NetworkRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageChange {
    pub from: String,
    pub to: String,
}

/// A network rule and the endpoints, `host:port` and any path, it gained or lost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRule {
    pub name: String,
    pub endpoints: Vec<String>,
}

/// A restart that runs: the version it creates and since when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Restart {
    pub version: ComputerVersion,
    pub since: chrono::DateTime<chrono::Utc>,
}

/// Why the last restart failed, when, and the version it was creating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestartFailure {
    pub cause: String,
    pub at: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

/// An update installed: from one version to another, and what it brought.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Updated {
    pub from: String,
    pub to: String,
    pub whats_new: WhatsNew,
}
