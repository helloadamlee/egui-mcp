use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IpcMessage {
    #[serde(rename = "screenshot_request")]
    ScreenshotRequest(ScreenshotRequest),

    #[serde(rename = "screenshot_response")]
    ScreenshotResponse(ScreenshotResponse),

    #[serde(rename = "click_at_request")]
    ClickAtRequest(ClickAtRequest),

    #[serde(rename = "click_at_response")]
    ClickAtResponse(ClickAtResponse),

    #[serde(rename = "keyboard_input_request")]
    KeyboardInputRequest(KeyboardInputRequest),

    #[serde(rename = "keyboard_input_response")]
    KeyboardInputResponse(KeyboardInputResponse),

    #[serde(rename = "mouse_move_request")]
    MouseMoveRequest(MouseMoveRequest),

    #[serde(rename = "mouse_move_response")]
    MouseMoveResponse(MouseMoveResponse),

    #[serde(rename = "scroll_request")]
    ScrollRequest(ScrollRequest),

    #[serde(rename = "scroll_response")]
    ScrollResponse(ScrollResponse),

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "error")]
    Error(ErrorMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotRequest {
    pub region: Option<Rect>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotResponse {
    pub image_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClickAtRequest {
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub double_click: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClickAtResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyboardInputRequest {
    pub key: String,
    pub pressed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyboardInputResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MouseMoveRequest {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MouseMoveResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollRequest {
    pub delta_x: i32,
    pub delta_y: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}
