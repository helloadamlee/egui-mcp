use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tracing::{error, info};

use egui_mcp_protocol::messages::IpcMessage;

const PIPE_NAME: &str = r"\\.\pipe\egui_mcp_client";

pub struct IpcServer {
    pipe: NamedPipeServer,
}

impl IpcServer {
    pub async fn new() -> Result<Self> {
        info!("Creating named pipe server at: {}", PIPE_NAME);

        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .create(PIPE_NAME)?;

        Ok(Self { pipe })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Waiting for connection to named pipe...");

        // Wait for a client to connect
        self.pipe.connect().await?;

        info!("Connected to client");

        // Handle communication with the client
        Self::handle_connection(&mut self.pipe).await?;

        Ok(())
    }

    async fn handle_connection(pipe: &mut NamedPipeServer) -> Result<()> {
        let mut buffer = Vec::new();

        loop {
            let mut chunk = [0; 4096];
            let n = pipe.read(&mut chunk).await?;

            if n == 0 {
                info!("Client disconnected");
                break;
            }

            buffer.extend_from_slice(&chunk[..n]);

            // Try to parse the message
            match Self::parse_message(&buffer) {
                Ok(Some(message)) => {
                    // Process the message
                    let response = Self::process_message(message).await?;

                    // Send the response
                    let response_bytes = serde_json::to_vec(&response)?;
                    pipe.write_all(&response_bytes).await?;
                    pipe.write_all(b"\n").await?; // Delimiter

                    buffer.clear();
                }
                Ok(None) => {
                    // Incomplete message, continue reading
                }
                Err(e) => {
                    error!("Failed to parse message: {}", e);
                    buffer.clear();
                }
            }
        }

        Ok(())
    }

    fn parse_message(buffer: &[u8]) -> Result<Option<IpcMessage>> {
        // Look for newline delimiter
        if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let message_bytes = &buffer[..pos];
            let message = serde_json::from_slice::<IpcMessage>(message_bytes)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    async fn process_message(message: IpcMessage) -> Result<IpcMessage> {
        info!("Received message: {:?}", message);

        match message {
            IpcMessage::Ping => Ok(IpcMessage::Pong),
            IpcMessage::ScreenshotRequest(req) => {
                // Handle screenshot request
                match crate::screenshot::capture_screen(req.region) {
                    Ok(image) => Ok(IpcMessage::ScreenshotResponse(image)),
                    Err(e) => Ok(IpcMessage::Error(
                        egui_mcp_protocol::messages::ErrorMessage {
                            message: "Failed to capture screenshot".to_string(),
                            details: Some(e.to_string()),
                        },
                    )),
                }
            }
            IpcMessage::ClickAtRequest(req) => {
                // Handle click request
                match crate::input::send_mouse_click(req.x, req.y, req.button, req.double_click) {
                    Ok(_) => Ok(IpcMessage::ClickAtResponse(
                        egui_mcp_protocol::messages::ClickAtResponse { success: true },
                    )),
                    Err(e) => Ok(IpcMessage::Error(
                        egui_mcp_protocol::messages::ErrorMessage {
                            message: "Failed to send mouse click".to_string(),
                            details: Some(e.to_string()),
                        },
                    )),
                }
            }
            IpcMessage::KeyboardInputRequest(req) => {
                // Handle keyboard input
                match crate::input::send_keyboard_input(&req.key, req.pressed) {
                    Ok(_) => Ok(IpcMessage::KeyboardInputResponse(
                        egui_mcp_protocol::messages::KeyboardInputResponse { success: true },
                    )),
                    Err(e) => Ok(IpcMessage::Error(
                        egui_mcp_protocol::messages::ErrorMessage {
                            message: "Failed to send keyboard input".to_string(),
                            details: Some(e.to_string()),
                        },
                    )),
                }
            }
            IpcMessage::MouseMoveRequest(req) => {
                // Handle mouse move
                match crate::input::send_mouse_move(req.x, req.y) {
                    Ok(_) => Ok(IpcMessage::MouseMoveResponse(
                        egui_mcp_protocol::messages::MouseMoveResponse { success: true },
                    )),
                    Err(e) => Ok(IpcMessage::Error(
                        egui_mcp_protocol::messages::ErrorMessage {
                            message: "Failed to move mouse".to_string(),
                            details: Some(e.to_string()),
                        },
                    )),
                }
            }
            IpcMessage::ScrollRequest(req) => {
                // Handle scroll
                match crate::input::send_scroll(req.delta_x, req.delta_y) {
                    Ok(_) => Ok(IpcMessage::ScrollResponse(
                        egui_mcp_protocol::messages::ScrollResponse { success: true },
                    )),
                    Err(e) => Ok(IpcMessage::Error(
                        egui_mcp_protocol::messages::ErrorMessage {
                            message: "Failed to scroll".to_string(),
                            details: Some(e.to_string()),
                        },
                    )),
                }
            }
            _ => Ok(IpcMessage::Error(
                egui_mcp_protocol::messages::ErrorMessage {
                    message: "Unsupported message type".to_string(),
                    details: None,
                },
            )),
        }
    }
}
