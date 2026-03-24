use std::sync::Arc;
use tokio::sync::Mutex;

pub mod highlight;
pub mod input;
pub mod ipc_server;
pub mod perf;
pub mod screenshot;

pub struct EguiMcpClient {
    ipc_server: Arc<Mutex<ipc_server::IpcServer>>,
}

impl EguiMcpClient {
    pub async fn new() -> anyhow::Result<Self> {
        let ipc_server = ipc_server::IpcServer::new().await?;

        Ok(Self {
            ipc_server: Arc::new(Mutex::new(ipc_server)),
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut server = self.ipc_server.lock().await;
        server.run().await
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub async fn init() -> anyhow::Result<EguiMcpClient> {
    EguiMcpClient::new().await
}
