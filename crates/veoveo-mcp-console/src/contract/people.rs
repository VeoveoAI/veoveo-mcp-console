//! An administrator ends a person's sessions, as when their access changes in the
//! identity provider: their next sign-in brings what it names then.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The arguments of `end_sessions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = EndedSessions, name = "end_sessions")]
pub struct EndSessionsRequest {
    /// The email the person signs in with.
    pub person: String,
}

/// What ending a person's sessions did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct EndedSessions {
    /// The person, by the email they sign in with.
    pub person: String,
    /// How many of their sessions ended.
    pub sessions_ended: u64,
    /// What happened to their Computer.
    pub computer: ComputerStop,
}

/// What ending a person's sessions did to their Computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ComputerStop {
    /// Stopped, its home kept; their next sign-in starts it again.
    Stopped,
    /// It was not running.
    NotRunning,
    /// A replacement held it, which starts without their sessions or Lore access.
    Busy,
    /// They have no Computer.
    None,
}
