//! The factory App, `ui://console/factory`: the factory in one page, as the person may see
//! it, over the server's own `factory` tool. Its look and its bridge to the host come from
//! `veoveo-hub-ui`, which a server puts into its document when it starts.
use std::sync::LazyLock;

use veoveo_hub_sdk::{
    apps::{AppHtml, AppUri, UiVisibility},
    hosting::{AppBuilder, RoutesBuilder},
};

use super::{ConsoleMcp, policy};

const APP_URI: &str = "ui://console/factory";
const APP_NAME: &str = "factory";
const APP_TITLE: &str = "Factory";
const APP_DESCRIPTION: &str = "What the factory serves: every server and branch, which one you \
     use, its Lore repository and links, its data environment and who owns it.";
const APP_DOCUMENT: &str = include_str!("../../assets/factory.html");
const APP_SCRIPT_MARKER: &str = "/*veoveoApp*/";

/// The tools the view calls, for the model and the App alike.
const APP_TOOLS: &[&str] = &["factory", "select", "watch", "logs"];

/// The App's document, with the bridge in place of its marker.
pub fn document() -> String {
    assert!(
        APP_DOCUMENT.contains(APP_SCRIPT_MARKER),
        "the App's document names where the bridge goes"
    );
    APP_DOCUMENT.replace(APP_SCRIPT_MARKER, veoveo_hub_ui::APP_SCRIPT)
}

/// The document as the SDK holds it, an immutable startup snapshot. An App's document is
/// loaded from a file, so it passes through one in the temporary folder.
pub(super) static APP: LazyLock<AppHtml> = LazyLock::new(|| {
    let path = std::env::temp_dir().join(format!("veoveo-app-{}-factory.html", std::process::id()));
    std::fs::write(&path, document()).expect("the factory App's document is written");
    let app = AppHtml::load(&path).expect("the factory App's document loads");
    let _ = std::fs::remove_file(&path);
    app
});

/// Declare the App, admitted as the tools are, and link the tools its view calls.
pub(super) fn register(routes: RoutesBuilder<ConsoleMcp>) -> RoutesBuilder<ConsoleMcp> {
    let app = AppUri::parse(APP_URI).expect("the factory App's address");
    routes
        .app(
            AppBuilder::new(app.clone(), APP_NAME)
                .title(APP_TITLE)
                .description(APP_DESCRIPTION)
                .policy(policy())
                .reader(|_: ConsoleMcp, _| async move { Ok(APP.clone()) }),
        )
        .app_tools(&app, APP_TOOLS, &[UiVisibility::Model, UiVisibility::App])
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_document_holds_the_bridge_and_loads_nothing_from_elsewhere() {
        let document = super::document();
        assert!(document.contains("const veoveoApp"));
        assert!(!document.contains(super::APP_SCRIPT_MARKER));
        assert!(document.contains("default-src 'none'"));
        for remote in ["src=\"http", "href=\"http", "url(http"] {
            assert!(!document.contains(remote), "{remote}");
        }
    }

    #[test]
    fn the_document_loads() {
        std::sync::LazyLock::force(&super::APP);
    }
}
