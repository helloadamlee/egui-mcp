use anyhow::Result;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("egui-mcp-server-win starting...");

    // Initialize tracing
    tracing_subscriber::fmt::init();

    let mut server = egui_mcp_server_win::EguiMcpServer::new().await?;
    server.run().await?;

    Ok(())
}
