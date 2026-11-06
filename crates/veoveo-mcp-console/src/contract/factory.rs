//! `factory`: the factory as the caller may see it, in one read. The types follow the
//! Hub's answer to `GET /factory/{profile}/instances`; fields the Hub adds later are not
//! reported until they are added here.
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ConsoleProvider, InstanceStatus};

/// How much of the factory to read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactoryScope {
    /// What your grants reach.
    #[default]
    Own,
    /// The whole factory of your tenant, for an administrator.
    Tenant,
}

impl FactoryScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Tenant => "tenant",
        }
    }
}

/// The arguments of `factory`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = FactoryState, name = "factory")]
pub struct FactoryRequest {
    /// `own`, the default: what your grants reach. `tenant`: the whole factory, which the
    /// Hub answers only to an administrator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<FactoryScope>,
}

/// The factory as you may see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct FactoryState {
    pub scope: FactoryScope,
    /// Who asked.
    pub caller: String,
    pub read_at: DateTime<Utc>,
    /// The branch that serves whoever selected none.
    pub base_branch: Option<String>,
    /// Which branch serves you each server.
    pub routes: Vec<FactoryRoute>,
    /// The servers the installation registered, not built from a branch.
    pub installed: Vec<String>,
    pub repositories: Vec<FactoryRepository>,
    pub databases: Vec<FactoryDatabase>,
    /// The people this answer names by principal who have signed in.
    pub people: Vec<FactoryPerson>,
    /// Everyone's selections, for `tenant`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selections: Option<Vec<FactorySelection>>,
}

/// Which branch serves you a server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryRoute {
    pub server: String,
    /// Null for a server the installation registered that no selection of yours names.
    pub branch: Option<String>,
    /// Whether you chose the branch, rather than the base branch serving you.
    pub selected: bool,
    /// Why this routes nothing, so that calls to the server fail with it.
    pub unavailable: Option<String>,
}

/// A Lore repository of the factory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryRepository {
    pub id: String,
    pub title: String,
    /// The server its branches serve, once one has registered.
    pub server: Option<String>,
    /// Whether you reach the repository itself; a person who reaches only one of its
    /// links sees that link and the repository's title.
    pub reached: bool,
    /// Whether the repository is linked into another's branch.
    pub linked: bool,
    /// Who the Hub follows the repository as; null while nobody watches it.
    pub watch: Option<super::WatchStatus>,
    /// Who may reach the repository.
    pub access: Vec<FactoryAccess>,
    pub instances: Vec<FactoryInstance>,
    /// Its live branches, read from Lore; empty while nobody watches it.
    pub branches: Vec<FactoryBranch>,
    /// Why Lore did not answer its branches.
    pub branches_error: Option<String>,
}

/// One person or group who may reach a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryAccess {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// `reader`, `member` or `owner`.
    pub level: super::AccessLevel,
}

/// An instance: one branch of the repository in its sandbox, or the runner of one of the
/// branch's links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryInstance {
    pub branch: String,
    pub link: Option<String>,
    /// `healthy`, `degraded` or `offline`.
    pub health: String,
    /// Null for a link whose repository you hold no grant on.
    pub detail: Option<FactoryInstanceDetail>,
}

/// What your grants reveal of an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryInstanceDetail {
    pub server: Option<String>,
    /// The database of its data environment.
    pub database: String,
    pub created_at: DateTime<Utc>,
    /// When its server last registered or resumed since its sandbox started.
    pub served_at: Option<DateTime<Utc>>,
    pub pusher: Option<String>,
    /// Whether the latest pusher still holds a grant on the repository: the instance's
    /// Lore token is issued as them, and Lore refuses one without a grant, so its next
    /// start cannot fetch the branch.
    pub pusher_holds_grant: bool,
    /// For a runner, the latest pusher of the linked repository's tracked branch.
    pub linked_pusher: Option<String>,
    pub status: InstanceStatus,
}

/// What the factory makes of a live branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactoryBranchState {
    /// It has an instance or runners.
    Served,
    /// Its latest revision has no executable `openshell/run`, so it does not ask to serve.
    NoRun,
    /// The Hub cannot name an instance after it.
    NameRefused,
    /// Its latest push could not be prepared; `message` says why.
    PrepareFailed,
    /// Its links could not run; `linkFailures` says why.
    LinksFailed,
    /// It asks to serve and nothing has failed yet.
    Pending,
    /// Lore did not answer for its latest revision; `message` says why.
    Unreadable,
}

/// A live branch of a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryBranch {
    pub name: String,
    /// Its latest revision; later than its instance's applied revision when the branch
    /// is ahead of what serves.
    pub latest: String,
    pub state: FactoryBranchState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub link_failures: Vec<FactoryLinkFailure>,
}

/// Why a link of a branch could not run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryLinkFailure {
    pub link: String,
    pub linked_repository: String,
    pub revision: String,
    pub message: String,
}

/// A database of a data environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryDatabase {
    pub name: String,
    pub bytes: u64,
    /// Whether it is the base branch's, which also keeps the Hub's own state.
    pub installation: bool,
}

/// A person the answer names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactoryPerson {
    pub principal: String,
    pub name: String,
    pub email: Option<String>,
}

/// One person's selection of a branch for a server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FactorySelection {
    pub principal: String,
    pub server: String,
    pub branch: String,
}
