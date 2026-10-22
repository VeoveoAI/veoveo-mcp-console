#[tokio::main]
async fn main() -> anyhow::Result<()> {
    veoveo_mcp_console::server::run().await
}
