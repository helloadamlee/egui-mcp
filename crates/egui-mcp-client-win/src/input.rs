use anyhow::{bail, Result};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use egui_mcp_protocol::messages::MouseButton;

/// Sends synthesized input, failing loudly when Windows refuses it.
///
/// `SendInput` returns how many events it actually queued. It silently drops
/// everything when the foreground window belongs to a more privileged process
/// (UIPI), so an unchecked call reports success for input that never landed.
fn dispatch_input(inputs: &[INPUT], action: &str) -> Result<()> {
    if inputs.is_empty() {
        return Ok(());
    }

    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };

    if sent as usize != inputs.len() {
        let last_error = unsafe { GetLastError() };
        if last_error.0 != 0 {
            bail!(
                "Windows accepted only {} of {} input events for {} (Win32 error code: {}). \
                 The target window is most likely running elevated; \
                 run this app as Administrator to match it.",
                sent,
                inputs.len(),
                action,
                last_error.0
            );
        } else {
            bail!(
                "Windows accepted only {} of {} input events for {}. \
                 The target window is most likely running elevated; \
                 run this app as Administrator to match it.",
                sent,
                inputs.len(),
                action
            );
        }
    }

    Ok(())
}

fn button_flags(button: MouseButton) -> (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    }
}

fn mouse_event(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, mouse_data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: mouse_data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Sends a single button-down or button-up event at the cursor's current position.
pub fn send_mouse_button(button: MouseButton, pressed: bool) -> Result<()> {
    let (down, up) = button_flags(button);
    let flags = if pressed { down } else { up };
    let inputs = [mouse_event(flags, 0, 0, 0)];
    dispatch_input(&inputs, "mouse button")
}

/// Converts screen pixels to the 0..65535 absolute range `SendInput` expects.
pub fn to_absolute(x: i32, y: i32) -> Result<(i32, i32)> {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    if screen_width <= 0 || screen_height <= 0 {
        bail!("Could not determine screen dimensions for coordinate conversion");
    }

    // i64 math: x * 65535 overflows i32 for coordinates past ~32767.
    let abs_x = (x as i64 * 65535 / screen_width as i64) as i32;
    let abs_y = (y as i64 * 65535 / screen_height as i64) as i32;

    Ok((abs_x, abs_y))
}

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

    dispatch_input(&inputs, "mouse click")
}

pub fn send_mouse_move(x: i32, y: i32) -> Result<()> {
    let (abs_x, abs_y) = to_absolute(x, y)?;

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

    dispatch_input(&inputs, "mouse move")
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

    dispatch_input(&inputs, "scroll")
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

    dispatch_input(&inputs, "keyboard input")
}

// Quick Win 7: Right Click at coordinates
pub fn send_right_click(x: i32, y: i32) -> Result<()> {
    send_mouse_click(x, y, MouseButton::Right, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_flags_maps_correct_mouse_buttons() {
        assert_eq!(
            button_flags(MouseButton::Left),
            (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)
        );
        assert_eq!(
            button_flags(MouseButton::Right),
            (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)
        );
        assert_eq!(
            button_flags(MouseButton::Middle),
            (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP)
        );
    }

    #[test]
    fn to_absolute_handles_zero_coordinates() {
        // (0, 0) should safely map to (0, 0)
        let res = to_absolute(0, 0);
        assert!(res.is_ok());
        let (abs_x, abs_y) = res.unwrap();
        assert_eq!(abs_x, 0);
        assert_eq!(abs_y, 0);
    }

    #[test]
    fn empty_dispatch_input_succeeds() {
        assert!(dispatch_input(&[], "empty").is_ok());
    }
}
