//! The `computer` tool's description names the installation's current Computer
//! version and its summary. Console follows it by reading the Hub's public
//! `GET /computer/version` about every 30 seconds, and changes the description, which
//! tells the Hub that the tool list changed, when the version does. A tool list is
//! shared by every caller, so the description names only the current version; the call
//! says whether the caller's Computer is behind.
use std::{sync::LazyLock, time::Duration};

use veoveo_hub_sdk::hosting::ToolDescriptions;

use super::Hub;
use crate::contract::{ComputerRequest, ComputerVersion};

/// How often console reads the current version.
pub const FOLLOW_INTERVAL: Duration = Duration::from_secs(30);

/// The descriptions console changes while it runs: the `computer` tool's.
pub static DESCRIPTIONS: LazyLock<ToolDescriptions> = LazyLock::new(ToolDescriptions::new);

const GUIDANCE: &str = "Call this at the start of a session; if an update is available or \
     was just installed, tell the person what's new in a line or two; restart only when \
     the person asks.";

/// The description before console has read the current version.
pub const UNKNOWN: &str = "Your Computer's status: the version it runs and the current one, \
     whether an update is available and what it brings, a restart that runs or the last \
     one's failure, and an update just installed. With `restart: true`, restart your \
     Computer at the current version; `~` is kept, running programs and `/tmp` are not. \
     Call this at the start of a session; if an update is available or was just \
     installed, tell the person what's new in a line or two; restart only when the \
     person asks.";

/// The description that names `current`.
pub fn describe(current: &ComputerVersion) -> String {
    let version = match &current.summary {
        Some(summary) => format!("`{}`: {summary}", current.version),
        None => format!("`{}`", current.version),
    };
    format!(
        "Your Computer's status: the version it runs and the current one, now {version}; \
         whether an update is available and what it brings, a restart that runs or the \
         last one's failure, and an update just installed. With `restart: true`, restart \
         your Computer at the current version; `~` is kept, running programs and `/tmp` \
         are not. {GUIDANCE}"
    )
}

/// Read the current version once and describe the tool by it. Returns whether the
/// description changed.
pub async fn refresh(hub: &Hub) -> Result<bool, super::hub::HubError> {
    let current = hub.current_computer_version().await?;
    Ok(DESCRIPTIONS.set::<ComputerRequest>(describe(&current)))
}

/// Follow the current version until the process ends. A read that fails keeps the
/// last description.
pub async fn follow(hub: Hub) {
    let mut interval = tokio::time::interval(FOLLOW_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        match refresh(&hub).await {
            Ok(true) => tracing::info!("the current Computer version changed"),
            Ok(false) => {}
            Err(error) => tracing::debug!(%error, "the current Computer version is unknown"),
        }
    }
}
