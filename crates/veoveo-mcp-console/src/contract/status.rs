//! `status`: why a branch of a repository serves or does not, as the Hub reports its
//! instance, or the runner of one of its links. The types follow the Hub's answer to
//! `GET /factory/{profile}/instance`; fields the Hub adds later are not reported until
//! they are added here.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The arguments of `status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = InstanceStatus, name = "status")]
pub struct StatusRequest {
    /// The Lore repository's identifier.
    pub repository: String,
    /// The branch whose instance to report.
    pub branch: String,
    /// A link at the top level of the branch, such as `team-a`, whose runner to report
    /// instead: the instance that serves the linked repository's tracked branch with the
    /// branch's program. Its status follows the caller's grant on the linked repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Seconds to wait for the applied revision to serve or fail before answering. At
    /// most 60 seconds are waited; absent or 0 answers at once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<u32>,
}

/// An instance: a repository's server running from one branch in its factory sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct InstanceStatus {
    /// The instance's deployment.
    pub deployment: String,
    /// The OpenShell workspace of the instance's sandbox.
    pub workspace: String,
    /// The name of the instance's OpenShell sandbox.
    pub sandbox_name: String,
    /// OpenShell's ID of the sandbox while it exists; its output in the log store
    /// carries it.
    pub sandbox_id: Option<String>,
    /// The sandbox as OpenShell reports it; null while OpenShell has none.
    pub sandbox: Option<SandboxStatus>,
    /// The revision the Hub last applied to the sandbox; a runner's is its branch's, the
    /// parent's.
    pub applied_revision: Option<String>,
    /// The applied revision while it serves: the sandbox is ready and the server
    /// registered or resumed after the Hub last restarted or created it. A runner is
    /// ready when it names the applied revision.
    pub serving_revision: Option<String>,
    /// Why the Hub's last step for the instance failed, until a later one succeeds.
    pub failed: Option<Failure>,
    /// Why the instance was created above a cap.
    pub warning: Option<String>,
    /// The providers the branch requested in `openshell/providers.yaml` that the
    /// installation does not grant its repository, so the sandbox runs without them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ungranted_providers: Vec<String>,
    /// For a runner: its link, the linked repository, the branch of it the link tracks
    /// and the revision of that branch the Hub last applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<RunnerStatus>,
    /// For a runner, who the Hub watches its linked repository as, and why the last
    /// subscription as them failed; none while nobody watches it, when the runner keeps
    /// serving what it last applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_watch: Option<WatchStatus>,
    /// For a runner, why the Hub could not run its link at the revision it names, such as
    /// a refused network rule of the linked repository, until a push runs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_failure: Option<RunnerFailure>,
}

/// What a runner serves beside its branch's program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunnerStatus {
    /// The link, by its name at the top level of the branch.
    pub link: String,
    /// The linked Lore repository.
    pub linked_repository: String,
    /// The branch of the linked repository the link tracks, where people push to change
    /// what the runner serves.
    pub tracked_branch: TrackedBranch,
    /// The revision of the tracked branch the Hub last applied.
    pub linked_revision: Option<String>,
}

/// A branch of a linked repository, by its Lore ID and its name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackedBranch {
    /// The branch's Lore ID.
    pub id: String,
    /// Its name, as the Hub last read it.
    pub name: String,
}

/// Who the Hub watches a repository as, and why the last subscription as them failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    /// The watcher's principal.
    pub watcher: String,
    /// Why the last subscription as the watcher failed; none while it holds.
    pub failed: Option<String>,
}

/// Why the Hub could not run a link of a branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunnerFailure {
    /// The linked repository the runner would serve.
    pub linked_repository: String,
    /// The revision the Hub could not run.
    pub revision: String,
    pub message: String,
}

/// The sandbox's lifecycle as OpenShell reports it. OpenShell restarts the sandbox's
/// main program whenever it exits, with backoff, so a failed fetch, build or server
/// start shows as its exit code and restart count; `logs` has its output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxStatus {
    pub phase: Phase,
    /// The main program's last exit code; signals count as 128 plus the signal number.
    pub exit_code: Option<i32>,
    /// Consecutive restarts in the current crash loop.
    pub restart_count: u32,
    /// OpenShell's readiness and lifecycle conditions, such as a scheduling failure.
    pub conditions: Vec<Condition>,
}

/// OpenShell's lifecycle phase of a sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Provisioning,
    Starting,
    Ready,
    Stopping,
    Stopped,
    Completed,
    Error,
    Deleting,
    Unknown,
}

/// One OpenShell condition, as OpenShell or the compute platform reported it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(rename = "type")]
    pub kind: String,
    pub status: String,
    pub reason: String,
    pub message: String,
}

/// The Hub's step for the instance that failed, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub stage: Stage,
    pub message: String,
}

/// The Hub's steps for an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Creating the instance would exceed an enforced cap.
    Capacity,
    /// Creating or migrating the database of the instance's data environment.
    Database,
    /// Building the pushed revision's environment image.
    Build,
    /// Creating the instance's provider or sandbox.
    Create,
    /// Setting the sandbox's policy, stopping and starting it.
    Restart,
    /// Deleting the sandbox, its providers and the instance.
    Delete,
}
