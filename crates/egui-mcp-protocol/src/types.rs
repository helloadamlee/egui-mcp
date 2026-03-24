use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UiElement {
    pub name: String,
    pub role: String,
    pub automation_id: String,
    pub bounds: Rect,
    pub children: Vec<UiElement>,
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

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Key {
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "escape")]
    Escape,
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "backspace")]
    Backspace,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "home")]
    Home,
    #[serde(rename = "end")]
    End,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "page_up")]
    PageUp,
    #[serde(rename = "page_down")]
    PageDown,
    #[serde(rename = "f1")]
    F1,
    #[serde(rename = "f2")]
    F2,
    #[serde(rename = "f3")]
    F3,
    #[serde(rename = "f4")]
    F4,
    #[serde(rename = "f5")]
    F5,
    #[serde(rename = "f6")]
    F6,
    #[serde(rename = "f7")]
    F7,
    #[serde(rename = "f8")]
    F8,
    #[serde(rename = "f9")]
    F9,
    #[serde(rename = "f10")]
    F10,
    #[serde(rename = "f11")]
    F11,
    #[serde(rename = "f12")]
    F12,
    #[serde(rename = "print_screen")]
    PrintScreen,
    #[serde(rename = "scroll_lock")]
    ScrollLock,
    #[serde(rename = "pause")]
    Pause,
    #[serde(rename = "insert")]
    Insert,
    #[serde(rename = "caps_lock")]
    CapsLock,
    #[serde(rename = "num_lock")]
    NumLock,
    #[serde(rename = "scroll_lock_key")]
    ScrollLockKey,
    #[serde(rename = "left_shift")]
    LeftShift,
    #[serde(rename = "right_shift")]
    RightShift,
    #[serde(rename = "left_ctrl")]
    LeftCtrl,
    #[serde(rename = "right_ctrl")]
    RightCtrl,
    #[serde(rename = "left_alt")]
    LeftAlt,
    #[serde(rename = "right_alt")]
    RightAlt,
    #[serde(rename = "left_win")]
    LeftWin,
    #[serde(rename = "right_win")]
    RightWin,
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "space")]
    Space,
    #[serde(rename = "0")]
    Key0,
    #[serde(rename = "1")]
    Key1,
    #[serde(rename = "2")]
    Key2,
    #[serde(rename = "3")]
    Key3,
    #[serde(rename = "4")]
    Key4,
    #[serde(rename = "5")]
    Key5,
    #[serde(rename = "6")]
    Key6,
    #[serde(rename = "7")]
    Key7,
    #[serde(rename = "8")]
    Key8,
    #[serde(rename = "9")]
    Key9,
    #[serde(rename = "a")]
    KeyA,
    #[serde(rename = "b")]
    KeyB,
    #[serde(rename = "c")]
    KeyC,
    #[serde(rename = "d")]
    KeyD,
    #[serde(rename = "e")]
    KeyE,
    #[serde(rename = "f")]
    KeyF,
    #[serde(rename = "g")]
    KeyG,
    #[serde(rename = "h")]
    KeyH,
    #[serde(rename = "i")]
    KeyI,
    #[serde(rename = "j")]
    KeyJ,
    #[serde(rename = "k")]
    KeyK,
    #[serde(rename = "l")]
    KeyL,
    #[serde(rename = "m")]
    KeyM,
    #[serde(rename = "n")]
    KeyN,
    #[serde(rename = "o")]
    KeyO,
    #[serde(rename = "p")]
    KeyP,
    #[serde(rename = "q")]
    KeyQ,
    #[serde(rename = "r")]
    KeyR,
    #[serde(rename = "s")]
    KeyS,
    #[serde(rename = "t")]
    KeyT,
    #[serde(rename = "u")]
    KeyU,
    #[serde(rename = "v")]
    KeyV,
    #[serde(rename = "w")]
    KeyW,
    #[serde(rename = "x")]
    KeyX,
    #[serde(rename = "y")]
    KeyY,
    #[serde(rename = "z")]
    KeyZ,
    #[serde(rename = "numpad0")]
    Numpad0,
    #[serde(rename = "numpad1")]
    Numpad1,
    #[serde(rename = "numpad2")]
    Numpad2,
    #[serde(rename = "numpad3")]
    Numpad3,
    #[serde(rename = "numpad4")]
    Numpad4,
    #[serde(rename = "numpad5")]
    Numpad5,
    #[serde(rename = "numpad6")]
    Numpad6,
    #[serde(rename = "numpad7")]
    Numpad7,
    #[serde(rename = "numpad8")]
    Numpad8,
    #[serde(rename = "numpad9")]
    Numpad9,
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "subtract")]
    Subtract,
    #[serde(rename = "multiply")]
    Multiply,
    #[serde(rename = "divide")]
    Divide,
    #[serde(rename = "decimal")]
    Decimal,
}
