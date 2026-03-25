use egui_mcp_protocol::messages::IpcMessage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tracing::info;

const PIPE_NAME: &str = r"\\.\pipe\egui_mcp_client";
#[allow(dead_code)]
const CONNECT_TIMEOUT_MS: u64 = 5000;

pub struct IpcClient {
    pipe: Option<NamedPipeClient>,
}

fn format_error_message(err: egui_mcp_protocol::messages::ErrorMessage) -> String {
    match err.details {
        Some(details) if !details.trim().is_empty() => format!("{}: {}", err.message, details),
        _ => err.message,
    }
}

impl IpcClient {
    pub async fn new() -> Result<Self, String> {
        info!("Connecting to named pipe: {}", PIPE_NAME);

        let pipe = ClientOptions::new()
            .open(PIPE_NAME)
            .map_err(|e| format!("Failed to open pipe: {}", e))?;

        info!("Successfully connected to egui app");

        Ok(Self { pipe: Some(pipe) })
    }

    async fn send_message(&mut self, message: IpcMessage) -> Result<IpcMessage, String> {
        let pipe = self.pipe.as_mut().ok_or("Not connected to pipe")?;

        // Serialize and send message
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        pipe.write_all(&message_bytes)
            .await
            .map_err(|e| format!("Failed to write to pipe: {}", e))?;
        pipe.write_all(b"\n")
            .await
            .map_err(|e| format!("Failed to write delimiter: {}", e))?;

        // Read response
        let mut buffer = Vec::new();
        let mut chunk = [0; 4096];

        loop {
            let n = pipe
                .read(&mut chunk)
                .await
                .map_err(|e| format!("Failed to read from pipe: {}", e))?;

            if n == 0 {
                return Err("Connection closed".to_string());
            }

            buffer.extend_from_slice(&chunk[..n]);

            // Check for newline delimiter
            if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let response_bytes = &buffer[..pos];
                let response = serde_json::from_slice::<IpcMessage>(response_bytes)
                    .map_err(|e| format!("Failed to deserialize response: {}", e))?;
                return Ok(response);
            }
        }
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

    pub async fn drag(&mut self, _x1: i32, _y1: i32, _x2: i32, _y2: i32) -> Result<(), String> {
        // Drag would need to be implemented by sending mouse_move to x1, then drag to x2
        // For now, simple implementation:
        self.hover(_x1, _y1).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.hover(_x2, _y2).await
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
