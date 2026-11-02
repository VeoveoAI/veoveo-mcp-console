//! The server's checked contract: slug, scheme, scope, documents and skills.
use std::sync::LazyLock;

use rmcp::model::{ServerCapabilities, ServerConfig};
use veoveo_hub_sdk::{
    docs::ServerDocs,
    server_contract::{McpServerContract, McpServerSetup},
    skills::ServerSkills,
};
use veoveo_types::ResourceScheme;

use crate::contract::{ConsoleProvider, ConsoleScope};

/// The crate documents embedded at build time and served under `console://docs`,
/// `console://docs/{doc_id}`, `console://contract` and the admin document routes.
pub(crate) static SERVER_DOCS: LazyLock<ServerDocs> =
    LazyLock::new(|| veoveo_hub_sdk::server_docs!(crate::SLUG));

/// The skills under `skills/`, embedded at build time and served at `skill://console/…`.
pub(crate) static SERVER_SKILLS: LazyLock<ServerSkills> =
    LazyLock::new(|| veoveo_hub_sdk::server_skills!(crate::SLUG));

pub struct ConsoleContract;

pub(crate) static SERVER_SETUP: LazyLock<McpServerSetup<ConsoleContract>> =
    LazyLock::new(|| McpServerSetup::new().expect("the console's MCP contract setup"));

impl McpServerContract for ConsoleContract {
    type Scope = ConsoleScope;
    type Server = super::ConsoleMcp;
    type Provider = ConsoleProvider;

    fn scheme() -> ResourceScheme {
        ResourceScheme::parse(crate::SLUG).expect("the slug is a URI scheme")
    }
    fn scopes() -> &'static [ConsoleScope] {
        ConsoleScope::ALL
    }
    fn documents() -> &'static ServerDocs {
        &SERVER_DOCS
    }
    fn skills() -> &'static ServerSkills {
        &SERVER_SKILLS
    }
    fn server_config() -> ServerConfig {
        server_config()
    }
}

fn server_config() -> ServerConfig {
    let mut capabilities = ServerCapabilities::builder()
        .enable_tools()
        .enable_resources()
        .enable_completions()
        .build();
    if let Some(registration) = super::declarations::REGISTRATION.get() {
        veoveo_hub_sdk::registration::mcp::advertise(
            &mut capabilities,
            registration,
            &super::requirements::AUTHORIZATION,
        );
    }
    let mut info = ServerConfig::default();
    info.capabilities = capabilities;
    info.server_info = rmcp::model::Implementation::new(crate::SLUG, env!("CARGO_PKG_VERSION"));
    info.instructions = Some(INSTRUCTIONS.to_owned());
    info
}

/// What the server is and which skills say how to use it.
const INSTRUCTIONS: &str = "The Hub's activities as you: your factory instances, the \
     branch that serves you each server, your repositories and groups, and your Computer. \
     Its skills say how: skill://console/serve-a-branch/SKILL.md, \
     skill://console/read-instance-logs/SKILL.md, skill://console/share-repositories/SKILL.md \
     and skill://console/update-the-computer/SKILL.md.";

#[cfg(test)]
mod tests {
    #[test]
    fn the_instructions_name_every_skill_the_server_serves() {
        let skills = &*super::SERVER_SKILLS;
        assert_eq!(skills.skills().len(), 4);
        for skill in skills.skills() {
            assert!(super::INSTRUCTIONS.contains(&skill.uri), "{}", skill.uri);
        }
    }

    #[test]
    fn the_setup_lists_only_the_hosts_documents() {
        let setup = &*super::SERVER_SETUP;
        assert!(
            setup
                .resources()
                .iter()
                .all(|resource| resource.address().as_str().starts_with("console://")),
        );
    }
}
