use egui_mcp_protocol::messages::IpcMessage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::time::{Duration, Instant};
use tracing::info;

const PIPE_NAME: &str = r"\\.\pipe\egui_mcp_client";
/// How long to keep retrying the initial connect before giving up. The egui
/// app may not have created its pipe yet, or may be momentarily busy
/// servicing a previous client.
const CONNECT_TIMEOUT_MS: u64 = 5000;
const RECONNECT_TIMEOUT_MS: u64 = 500;
const CONNECT_RETRY_INTERVAL_MS: u64 = 100;
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct IpcClient {
    pipe: Option<NamedPipeClient>,
    read_buffer: Vec<u8>,
}

fn format_error_message(err: egui_mcp_protocol::messages::ErrorMessage) -> String {
    match err.details {
        Some(details) if !details.trim().is_empty() => format!("{}: {}", err.message, details),
        _ => err.message,
    }
}

impl IpcClient {
    pub async fn new() -> Result<Self, String> {
        Self::connect_with_timeout(CONNECT_TIMEOUT_MS).await
    }

    pub async fn connect_fast() -> Result<Self, String> {
        Self::connect_with_timeout(RECONNECT_TIMEOUT_MS).await
    }

    pub async fn connect_with_timeout(timeout_ms: u64) -> Result<Self, String> {
        info!("Connecting to named pipe: {}", PIPE_NAME);

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            match ClientOptions::new().open(PIPE_NAME) {
                Ok(pipe) => {
                    info!("Successfully connected to egui app");
                    return Ok(Self {
                        pipe: Some(pipe),
                        read_buffer: Vec::new(),
                    });
                }
                Err(e) => {
                    if Instant::now() >= deadline {
                        return Err(format!("Failed to open pipe: {}", e));
                    }
                    tokio::time::sleep(Duration::from_millis(CONNECT_RETRY_INTERVAL_MS)).await;
                }
            }
        }
    }

    /// Returns false once the connection has been observed to be broken.
    /// The pipe is otherwise assumed healthy between calls.
    pub fn is_connected(&self) -> bool {
        self.pipe.is_some()
    }

    async fn send_message(&mut self, message: IpcMessage) -> Result<IpcMessage, String> {
        // Serialize before touching the pipe so a bad message never tears
        // down an otherwise-healthy connection.
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        if self.pipe.is_none() {
            return Err("Not connected to pipe".to_string());
        }

        let result = self.exchange(&message_bytes).await;

        // Any failure here (write error, read error, a peer that closed the
        // pipe, or a garbled response) means the stream can no longer be
        // trusted. Drop `pipe` and leave self.pipe as None so the next call
        // knows to reconnect instead of continuing to use a dead handle.
        if result.is_err() {
            self.pipe = None;
            self.read_buffer.clear();
        }

        result
    }

    async fn exchange(&mut self, message_bytes: &[u8]) -> Result<IpcMessage, String> {
        let pipe = self.pipe.as_mut().ok_or("Not connected to pipe")?;

        tokio::time::timeout(EXCHANGE_TIMEOUT, async {
            pipe.write_all(message_bytes)
                .await
                .map_err(|e| format!("Failed to write to pipe: {}", e))?;
            pipe.write_all(b"\n")
                .await
                .map_err(|e| format!("Failed to write delimiter: {}", e))?;

            let mut chunk = [0; 4096];

            loop {
                // Check if a full line already exists in self.read_buffer
                while let Some(pos) = self.read_buffer.iter().position(|&b| b == b'\n') {
                    let line_bytes: Vec<u8> = self.read_buffer.drain(..=pos).collect();
                    let mut line = &line_bytes[..line_bytes.len() - 1]; // trim \n
                    if line.ends_with(b"\r") {
                        line = &line[..line.len() - 1]; // trim \r
                    }

                    if line.is_empty() {
                        continue;
                    }

                    return serde_json::from_slice::<IpcMessage>(line)
                        .map_err(|e| format!("Failed to deserialize response: {}", e));
                }

                let n = pipe
                    .read(&mut chunk)
                    .await
                    .map_err(|e| format!("Failed to read from pipe: {}", e))?;

                if n == 0 {
                    return Err("Connection closed".to_string());
                }

                self.read_buffer.extend_from_slice(&chunk[..n]);
            }
        })
        .await
        .map_err(|_| "IPC request timed out after 10 seconds".to_string())?
    }

    pub async fn click_at(&mut self, x: i32, y: i32) -> Result<(), String> {
        let request = IpcMessage::ClickAtRequest(egui_mcp_protocol::messages::ClickAtRequest {
            x,
            y,
            button: egui_mcp_protocol::messages::MouseButton::Left,
            double_click: false,
        });

        match self.send_message(request).await? {
            IpcMessage::ClickAtResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Click failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn right_click_at(&mut self, x: i32, y: i32) -> Result<(), String> {
        let request = IpcMessage::ClickAtRequest(egui_mcp_protocol::messages::ClickAtRequest {
            x,
            y,
            button: egui_mcp_protocol::messages::MouseButton::Right,
            double_click: false,
        });

        match self.send_message(request).await? {
            IpcMessage::ClickAtResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Right click failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn double_click(&mut self, x: i32, y: i32) -> Result<(), String> {
        let request = IpcMessage::ClickAtRequest(egui_mcp_protocol::messages::ClickAtRequest {
            x,
            y,
            button: egui_mcp_protocol::messages::MouseButton::Left,
            double_click: true,
        });

        match self.send_message(request).await? {
            IpcMessage::ClickAtResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Double click failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn hover(&mut self, x: i32, y: i32) -> Result<(), String> {
        let request =
            IpcMessage::MouseMoveRequest(egui_mcp_protocol::messages::MouseMoveRequest { x, y });

        match self.send_message(request).await? {
            IpcMessage::MouseMoveResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Hover failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn drag(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), String> {
        let request =
            IpcMessage::DragRequest(egui_mcp_protocol::messages::DragRequest::new(x1, y1, x2, y2));

        match self.send_message(request).await? {
            IpcMessage::DragResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Drag failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn keyboard_input(&mut self, key: &str) -> Result<(), String> {
        // Send key down
        let request_down =
            IpcMessage::KeyboardInputRequest(egui_mcp_protocol::messages::KeyboardInputRequest {
                key: key.to_string(),
                pressed: true,
            });

        match self.send_message(request_down).await? {
            IpcMessage::KeyboardInputResponse(resp) => {
                if !resp.success {
                    return Err("Keyboard input (down) failed".to_string());
                }
            }
            IpcMessage::Error(err) => return Err(err.message),
            _ => return Err("Unexpected response".to_string()),
        }

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Send key up
        let request_up =
            IpcMessage::KeyboardInputRequest(egui_mcp_protocol::messages::KeyboardInputRequest {
                key: key.to_string(),
                pressed: false,
            });

        match self.send_message(request_up).await? {
            IpcMessage::KeyboardInputResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Keyboard input (up) failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn scroll(&mut self, delta_x: i32, delta_y: i32) -> Result<(), String> {
        let request = IpcMessage::ScrollRequest(egui_mcp_protocol::messages::ScrollRequest {
            delta_x,
            delta_y,
        });

        match self.send_message(request).await? {
            IpcMessage::ScrollResponse(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err("Scroll failed".to_string())
                }
            }
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn take_screenshot(&mut self) -> Result<String, String> {
        let request =
            IpcMessage::ScreenshotRequest(egui_mcp_protocol::messages::ScreenshotRequest {
                region: None,
            });

        match self.send_message(request).await? {
            IpcMessage::ScreenshotResponse(resp) => {
                // Convert image bytes to base64
                use base64::{engine::general_purpose, Engine as _};
                Ok(general_purpose::STANDARD.encode(&resp.image_data))
            }
            IpcMessage::Error(err) => Err(format_error_message(err)),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn ping(&mut self) -> Result<(), String> {
        let request = IpcMessage::Ping;

        match self.send_message(request).await? {
            IpcMessage::Pong => Ok(()),
            IpcMessage::Error(err) => Err(err.message),
            _ => Err("Unexpected response".to_string()),
        }
    }

    pub async fn screenshot_region(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<String, String> {
        let request =
            IpcMessage::ScreenshotRequest(egui_mcp_protocol::messages::ScreenshotRequest {
                region: Some(egui_mcp_protocol::messages::Rect::new(x, y, width, height)),
            });

        match self.send_message(request).await? {
            IpcMessage::ScreenshotResponse(resp) => {
                // Convert image bytes to base64
                use base64::{engine::general_purpose, Engine as _};
                Ok(general_purpose::STANDARD.encode(&resp.image_data))
            }
            IpcMessage::Error(err) => Err(format_error_message(err)),
            _ => Err("Unexpected response".to_string()),
        }
    }
}
