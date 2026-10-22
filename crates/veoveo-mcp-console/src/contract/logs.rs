//! `logs`: an instance's output and OpenShell's network denials for it, from the log
//! store, bounded.
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The lines `logs` returns when the request names no limit.
pub const DEFAULT_LOG_LINES: u32 = 200;
/// The most lines `logs` returns.
pub const MAX_LOG_LINES: u32 = 1000;
/// The longest line `logs` returns, in bytes; a longer one is cut and ends in `…`.
pub const MAX_LOG_LINE_BYTES: usize = 4096;

/// The arguments of `logs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Logs, name = "logs")]
pub struct LogsRequest {
    /// The Lore repository's identifier.
    pub repository: String,
    /// The branch whose instance's output to read.
    pub branch: String,
    /// Only lines from this time on, in RFC 3339; absent, the latest lines the log store
    /// keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<DateTime<Utc>>,
    /// The most lines to return, the latest ones: 200 when absent, at most 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// An instance's output: everything its sandbox's main program wrote — the Lore sync,
/// the build and the server — with OpenShell's network denials for the sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Logs {
    /// The instance's sandbox, as `status` reports it.
    pub sandbox_name: String,
    /// OpenShell's ID of the sandbox whose lines these are; null while OpenShell has
    /// no sandbox for the instance, when there are no lines.
    pub sandbox_id: Option<String>,
    /// The latest lines, oldest first.
    pub lines: Vec<LogLine>,
    /// Whether older lines were left out to keep within the limit.
    pub truncated: bool,
}

/// One line of an instance's output, or one network denial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogLine {
    pub time: DateTime<Utc>,
    pub source: LogSource,
    pub text: String,
}

/// Where a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    /// The main program's standard output.
    Stdout,
    /// The main program's standard error.
    Stderr,
    /// A network request of the sandbox that OpenShell denied, from its supervisor.
    Denied,
}
