use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // stdout carries the JSON-RPC stream, so every diagnostic must go to stderr.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let mut server = egui_mcp_server_win::EguiMcpServer::new().await?;
    server.run().await?;

    Ok(())
}
