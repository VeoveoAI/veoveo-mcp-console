//! Console, the MCP provider of the Hub's activities for agents. It holds no authority
//! of its own: every tool calls the Hub as the calling user, and the Hub decides. The
//! `contract` feature holds the server's public types; the default `server` feature
//! adds the hosted server.

#[cfg(feature = "contract")]
pub mod contract;

#[cfg(feature = "server")]
pub mod mcp;
#[cfg(feature = "server")]
pub mod server;

/// The server's slug: its name in the Hub, its schema and URI scheme, and the prefix of
/// its permissions and of its tools as agents see them, `console__{tool}`. Every other
/// spelling of the slug derives from this one.
pub const SLUG: &str = "console";

/// The component name, taken from the crate name.
pub fn component() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// The server's slug as the SDK's type.
#[cfg(feature = "contract")]
pub fn server() -> veoveo_types::ServerSlug {
    veoveo_types::ServerSlug::parse(SLUG).expect("the server's slug")
}

#[cfg(test)]
mod tests {
    #[test]
    fn component_is_the_crate_name() {
        assert_eq!(super::component(), "veoveo-mcp-console");
    }
}
