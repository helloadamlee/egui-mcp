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
        // Serve connections one after another for the life of the process.
        // A client disconnecting is a normal event, not a fatal one — reset
        // the pipe instance and wait for the next connection.
        loop {
            info!("Waiting for connection to named pipe...");
            self.pipe.connect().await?;
            info!("Connected to client");

            if let Err(e) = Self::handle_connection(&mut self.pipe).await {
                error!("IPC connection ended with an error: {}", e);
            }

            self.pipe.disconnect()?;
        }
    }
    async fn handle_connection(pipe: &mut NamedPipeServer) -> Result<()> {
        let mut buffer = Vec::new();

        loop {
            // Drain and answer every complete message already buffered
            // before reading more — a single read() can return more than
            // one pipelined message, and only handling the first would
            // silently drop the rest.
            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let message_bytes: Vec<u8> = buffer.drain(..=pos).collect();
                let mut line = &message_bytes[..message_bytes.len() - 1]; // trim \n
                if line.ends_with(b"\r") {
                    line = &line[..line.len() - 1]; // trim \r
                }

                if line.is_empty() {
                    continue;
                }

                let response = match serde_json::from_slice::<IpcMessage>(line) {
                    Ok(message) => match Self::process_message(message).await {
                        Ok(resp) => resp,
                        Err(e) => IpcMessage::Error(egui_mcp_protocol::messages::ErrorMessage {
                            message: "Internal server error processing IPC message".to_string(),
                            details: Some(e.to_string()),
                        }),
                    },
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                        IpcMessage::Error(egui_mcp_protocol::messages::ErrorMessage {
                            message: format!("Failed to parse IPC message: {}", e),
                            details: None,
                        })
                    }
                };

                let response_bytes = serde_json::to_vec(&response)?;
                pipe.write_all(&response_bytes).await?;
                pipe.write_all(b"\n").await?; // Delimiter
            }

            let mut chunk = [0; 4096];
            let n = pipe.read(&mut chunk).await?;

            if n == 0 {
                info!("Client disconnected");
                break;
            }

            buffer.extend_from_slice(&chunk[..n]);
        }

        Ok(())
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
            IpcMessage::DragRequest(req) => {
                // Handle drag: a press-move-release gesture performed as one unit,
                // so the button stays down for every intermediate move.
                match Self::perform_drag(&req).await {
                    Ok(_) => Ok(IpcMessage::DragResponse(
                        egui_mcp_protocol::messages::DragResponse { success: true },
                    )),
                    Err(e) => Ok(IpcMessage::Error(
                        egui_mcp_protocol::messages::ErrorMessage {
                            message: "Failed to drag".to_string(),
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

    /// Performs a drag as a single press-move-release gesture, keeping the
    /// button held down through every intermediate move so the target
    /// application sees a continuous drag rather than a move plus a click.
    async fn perform_drag(req: &egui_mcp_protocol::messages::DragRequest) -> Result<()> {
        crate::input::send_mouse_move(req.x1, req.y1)?;
        crate::input::send_mouse_button(req.button, true)?;

        let result = async {
            let steps = req.steps.max(1);
            for step in 1..=steps {
                let t = step as f64 / steps as f64;
                let x = req.x1 + ((req.x2 - req.x1) as f64 * t).round() as i32;
                let y = req.y1 + ((req.y2 - req.y1) as f64 * t).round() as i32;
                crate::input::send_mouse_move(x, y)?;

                if req.step_delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(req.step_delay_ms)).await;
                }
            }
            Ok(())
        }
        .await;

        // Always release the mouse button even if intermediate steps fail.
        let _ = crate::input::send_mouse_button(req.button, false);
        result
    }
}
