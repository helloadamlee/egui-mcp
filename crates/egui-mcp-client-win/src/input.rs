use anyhow::Result;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use egui_mcp_protocol::messages::MouseButton;

pub fn send_mouse_click(x: i32, y: i32, button: MouseButton, double_click: bool) -> Result<()> {
    // Move mouse to position first
    send_mouse_move(x, y)?;

    // Send click(s)
    let down = match button {
        MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
        MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
        MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
    };

    let up = match button {
        MouseButton::Left => MOUSEEVENTF_LEFTUP,
        MouseButton::Right => MOUSEEVENTF_RIGHTUP,
        MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
    };

    let mut inputs = vec![
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    if double_click {
        inputs.push(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    Ok(())
}

pub fn send_mouse_move(x: i32, y: i32) -> Result<()> {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    // Convert to absolute coordinates (0-65535 range)
    let abs_x = (x * 65535 / screen_width) as i32;
    let abs_y = (y * 65535 / screen_height) as i32;

    let inputs = [INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: abs_x,
                dy: abs_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    Ok(())
}

pub fn send_scroll(_delta_x: i32, delta_y: i32) -> Result<()> {
    let mouse_data = (delta_y * 120) as u32; // 120 is the standard scroll unit

    let inputs = [INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: mouse_data,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    Ok(())
}

pub fn send_keyboard_input(key: &str, pressed: bool) -> Result<()> {
    // For now, this is a simple implementation
    // We'll expand this to handle more keys later

    let vk_code = match key.to_lowercase().as_str() {
        "enter" => VK_RETURN,
        "escape" => VK_ESCAPE,
        "tab" => VK_TAB,
        "backspace" => VK_BACK,
        "delete" => VK_DELETE,
        "home" => VK_HOME,
        "end" => VK_END,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "page_up" => VK_PRIOR,
        "page_down" => VK_NEXT,
        "space" => VK_SPACE,
        "a" => VK_A,
        "b" => VK_B,
        "c" => VK_C,
        "d" => VK_D,
        "e" => VK_E,
        "f" => VK_F,
        "g" => VK_G,
        "h" => VK_H,
        "i" => VK_I,
        "j" => VK_J,
        "k" => VK_K,
        "l" => VK_L,
        "m" => VK_M,
        "n" => VK_N,
        "o" => VK_O,
        "p" => VK_P,
        "q" => VK_Q,
        "r" => VK_R,
        "s" => VK_S,
        "t" => VK_T,
        "u" => VK_U,
        "v" => VK_V,
        "w" => VK_W,
        "x" => VK_X,
        "y" => VK_Y,
        "z" => VK_Z,
        "0" => VK_0,
        "1" => VK_1,
        "2" => VK_2,
        "3" => VK_3,
        "4" => VK_4,
        "5" => VK_5,
        "6" => VK_6,
        "7" => VK_7,
        "8" => VK_8,
        "9" => VK_9,
        _ => return Err(anyhow::anyhow!("Unsupported key: {}", key)),
    };

    let inputs = [INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk_code,
                wScan: 0,
                dwFlags: if pressed {
                    KEYBD_EVENT_FLAGS(0)
                } else {
                    KEYEVENTF_KEYUP
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    Ok(())
}

// Quick Win 7: Right Click at coordinates
pub fn send_right_click(x: i32, y: i32) -> Result<()> {
    send_mouse_click(x, y, MouseButton::Right, false)
}
