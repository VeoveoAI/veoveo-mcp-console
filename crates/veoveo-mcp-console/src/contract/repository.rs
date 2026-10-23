//! `create_repository`: a Lore repository the caller creates and owns, as the Hub
//! answers `POST /lore/{profile}/repositories`.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConsoleProvider;

/// The arguments of `create_repository`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[veoveo_hub_sdk::tool(provider = ConsoleProvider, output = Repository, name = "create_repository")]
pub struct CreateRepositoryRequest {
    /// The repository's name, which no other repository in the installation's Lore
    /// holds.
    pub name: String,
    /// Only to finish a creation the Hub answered as created but not registered: the
    /// `id` it answered. Omit it to create a new repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// A Lore repository you own: your Computers' Lore access carries it once this
/// answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(veoveo_hub_sdk::SingleOutput)]
pub struct Repository {
    /// The repository's identifier, which `watch`, `status` and `logs` take.
    pub id: String,
    pub name: String,
    /// The URL to clone it from: the Lore remote followed by `/{id}`.
    pub remote: String,
}
