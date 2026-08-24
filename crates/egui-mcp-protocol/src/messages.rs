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

    #[serde(rename = "drag_request")]
    DragRequest(DragRequest),

    #[serde(rename = "drag_response")]
    DragResponse(DragResponse),

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

/// A press-move-release gesture performed as one unit by the client.
///
/// The whole gesture has to happen on the app side: sending a bare move, then a
/// separate press, lets other input interleave and drops the button state that
/// makes a drag a drag.
#[derive(Debug, Serialize, Deserialize)]
pub struct DragRequest {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub button: MouseButton,
    /// Intermediate move events between press and release. Applications that
    /// track motion need several; one long jump often reads as a click.
    pub steps: u32,
    /// Pause between successive move events, in milliseconds.
    pub step_delay_ms: u64,
}

impl DragRequest {
    pub const DEFAULT_STEPS: u32 = 12;
    pub const DEFAULT_STEP_DELAY_MS: u64 = 12;

    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            button: MouseButton::Left,
            steps: Self::DEFAULT_STEPS,
            step_delay_ms: Self::DEFAULT_STEP_DELAY_MS,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DragResponse {
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
