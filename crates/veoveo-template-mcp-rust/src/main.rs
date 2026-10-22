#[tokio::main]
async fn main() -> anyhow::Result<()> {
    veoveo_template_mcp_rust::server::run().await
}
