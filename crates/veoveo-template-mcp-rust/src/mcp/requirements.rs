//! Registration uses the ready route table directly.
use super::setup::TemplateContract;
use std::sync::LazyLock;
use veoveo_hub_sdk::{registration::AuthorizationDeclaration, server_contract::McpServerContract};
pub static AUTHORIZATION: LazyLock<AuthorizationDeclaration> = LazyLock::new(|| {
    let declaration = super::ROUTES.authorization();
    declaration
        .check(&crate::server(), &TemplateContract::scheme())
        .expect("Template route authorization");
    declaration
});
