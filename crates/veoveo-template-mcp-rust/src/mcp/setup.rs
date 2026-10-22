//! The server's checked contract: slug, scheme, scope, documents, the fixed resources it
//! lists and the templates it declares.
use std::sync::LazyLock;

use rmcp::model::{ServerCapabilities, ServerConfig};
use veoveo_hub_sdk::{
    docs::ServerDocs,
    server_contract::{McpServerContract, McpServerSetup},
};
use veoveo_types::ResourceScheme;

use crate::contract::{TemplateProvider, TemplateScope};

/// The crate documents embedded at build time and served under `template://docs`,
/// `template://docs/{doc_id}`, `template://contract` and the admin document routes.
pub(crate) static SERVER_DOCS: LazyLock<ServerDocs> =
    LazyLock::new(|| veoveo_hub_sdk::server_docs!("template"));

pub struct TemplateContract;

pub(crate) static SERVER_SETUP: LazyLock<McpServerSetup<TemplateContract>> =
    LazyLock::new(|| McpServerSetup::new().expect("the template's MCP contract setup"));

impl McpServerContract for TemplateContract {
    type Scope = TemplateScope;
    type Server = super::TemplateMcp;
    type Provider = TemplateProvider;

    fn scheme() -> ResourceScheme {
        ResourceScheme::parse("template").expect("declared template scheme")
    }
    fn scopes() -> &'static [TemplateScope] {
        <TemplateScope as veoveo_types::Vocabulary>::ALL
    }
    fn documents() -> &'static ServerDocs {
        &SERVER_DOCS
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
    info.server_info = rmcp::model::Implementation::new("template", env!("CARGO_PKG_VERSION"));
    info.instructions = Some(
        "Keep a list of named items for your tenant. Call `create_item` with a name, read an \
         item at its `resultUri`, and list your items at `template://items`."
            .to_owned(),
    );
    info
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_setup_round_trips_every_resource_and_template() {
        let setup = &*super::SERVER_SETUP;
        assert_eq!(setup.resources().len(), 5);
        assert_eq!(setup.resource_templates().len(), 3);
    }
}
