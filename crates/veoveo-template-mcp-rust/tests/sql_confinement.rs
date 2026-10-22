//! The server's SQL names only the `template` schema. Core state comes through
//! veoveo-hub-sdk and core's runtime API.

#[test]
fn sql_names_only_the_template_schema() {
    veoveo_hub_sdk::testing::assert_sql_confined(env!("CARGO_MANIFEST_DIR"), "template");
}
