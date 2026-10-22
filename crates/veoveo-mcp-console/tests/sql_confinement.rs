//! The server holds no SQL of its own; the check keeps it so. Core state comes through
//! veoveo-hub-sdk and core's runtime API.

#[test]
fn sql_names_only_the_consoles_schema() {
    veoveo_hub_sdk::testing::assert_sql_confined(
        env!("CARGO_MANIFEST_DIR"),
        veoveo_mcp_console::SLUG,
    );
}
