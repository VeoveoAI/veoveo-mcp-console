//! `repository_access`, `share_repository`, `groups`, `create_group`, `set_group_member`
//! and `delete_group`: who may reach a repository, and the groups people share
//! repositories with, as the Hub answers its `/lore/{profile}` access and group routes.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The arguments of `repository_access`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = RepositoryAccess, name = "repository_access")]
pub struct RepositoryAccessRequest {
    /// The Lore repository's identifier.
    pub repository: String,
}

/// The arguments of `share_repository`: one person or one group, and the access to set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = RepositoryAccess, name = "share_repository")]
pub struct ShareRepositoryRequest {
    /// The Lore repository's identifier.
    pub repository: String,
    /// The email the person signs in with. Give this or `group`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<String>,
    /// The group's name. Give this or `person`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub access: ShareAccess,
}

/// The access `share_repository` sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShareAccess {
    /// Clone, read and push.
    Member,
    /// A member who also shares the repository.
    Owner,
    /// Remove this person's or group's access.
    None,
}

/// Who may reach a repository: each person and group assigned it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct RepositoryAccess {
    pub repository: String,
    pub access: Vec<AccessEntry>,
}

/// One person or group with access to a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessEntry {
    #[serde(flatten)]
    pub subject: Subject,
    pub access: AccessLevel,
    /// Lore's named privileges the assignment holds, such as `owner`.
    pub actions: Vec<String>,
}

/// The person or the group an assignment names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    Person(Person),
    Group(GroupName),
}

/// A person, by the email and name of their last sign-in; both are null for someone who
/// has not signed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupName {
    pub name: String,
}

/// The access an assignment gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Member,
    Owner,
}

/// The arguments of `groups`, which takes none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Groups, name = "groups")]
pub struct GroupsRequest {}

/// The groups you belong to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Groups {
    pub groups: Vec<Group>,
}

/// The arguments of `create_group`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Group, name = "create_group")]
pub struct CreateGroupRequest {
    /// The group's name: 1 to 64 lowercase letters, digits and `-`, starting with a
    /// letter, which no other group of your tenant holds.
    pub name: String,
}

/// The arguments of `set_group_member`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Group, name = "set_group_member")]
pub struct SetGroupMemberRequest {
    /// The group's name.
    pub group: String,
    /// The email the person signs in with.
    pub person: String,
    pub role: MemberRole,
}

/// The arguments of `delete_group`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Groups, name = "delete_group")]
pub struct DeleteGroupRequest {
    /// The group's name.
    pub group: String,
}

/// The role `set_group_member` sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    /// Reaches every repository shared with the group.
    Member,
    /// A member who also manages the group's members.
    Owner,
    /// Remove the person from the group.
    None,
}

/// A group and its members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Group {
    /// The ID a grant outside Lore names the group by, such as a grant of an artifact.
    pub id: String,
    pub name: String,
    pub members: Vec<Member>,
}

/// A member of a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: GroupRole,
}

/// A member's role in a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupRole {
    Member,
    Owner,
}
