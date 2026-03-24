use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use tracing::{error, info};

mod ipc;
mod tools;
mod uia_client;

pub use ipc::IpcClient;
pub use uia_client::{ElementInfo, Rect as UiaRect, UiaClient};

const WINDOW_TITLE: &str = "egui-mcp Demo";

fn required_non_empty_string_param<'a>(
    params: &'a Value,
    key: &str,
) -> std::result::Result<&'a str, String> {
    let Some(raw_value) = params.get(key) else {
        return Err(format!("Missing required parameter '{}'", key));
    };

    let Some(value) = raw_value.as_str() else {
        return Err(format!("Parameter '{}' must be a string", key));
    };

    let value = value.trim();
    if value.is_empty() {
        return Err(format!("Parameter '{}' cannot be empty", key));
    }

    Ok(value)
}

fn flash_points_for_rect(rect: &UiaRect) -> Vec<(i32, i32)> {
    let left = rect.x;
    let top = rect.y;
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    let center_x = rect.x + (rect.width / 2);
    let center_y = rect.y + (rect.height / 2);

    vec![
        (left, top),
        (right, top),
        (right, bottom),
        (left, bottom),
        (center_x, center_y),
    ]
}

/// Simple MCP protocol implementation
pub struct McpServer<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    ipc_client: Option<IpcClient>,
    uia_client: Option<UiaClient>,
}

impl<R: BufRead, W: Write> McpServer<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            ipc_client: None,
            uia_client: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        // Connect to IPC
        info!("Connecting to egui app via named pipe...");
        match IpcClient::new().await {
            Ok(client) => {
                self.ipc_client = Some(client);
                info!("Connected to egui app successfully via IPC");
            }
            Err(e) => {
                error!(
                    "Failed to connect to egui app via IPC: {}. IPC tools won't work.",
                    e
                );
            }
        }

        // Initialize UI Automation
        info!("Initializing UI Automation client...");
        match UiaClient::new().await {
            Ok(client) => {
                match client.find_window_by_title(WINDOW_TITLE).await {
                    Ok(_) => {
                        info!(
                            "UI Automation connected to target window '{}'",
                            WINDOW_TITLE
                        );
                    }
                    Err(err) => {
                        info!(
                            "UI Automation initialized, but target window '{}' is not available yet: {}",
                            WINDOW_TITLE, err
                        );
                    }
                }
                self.uia_client = Some(client);
                info!("UI Automation client initialized successfully");
            }
            Err(e) => {
                error!(
                    "Failed to initialize UI Automation: {}. UI tree tools won't work.",
                    e
                );
            }
        }

        Ok(())
    }

    pub fn read_message(&mut self) -> Result<Option<Value>> {
        let mut buffer = String::new();
        let bytes_read = self.reader.read_line(&mut buffer)?;

        if bytes_read == 0 || buffer.trim().is_empty() {
            return Ok(None);
        }

        let message: Value = serde_json::from_str(&buffer.trim())?;
        Ok(Some(message))
    }

    pub fn write_response(&mut self, id: i64, result: Value) -> Result<()> {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        writeln!(self.writer, "{}", response)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn write_error(&mut self, id: i64, code: i32, message: String) -> Result<()> {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        });
        writeln!(self.writer, "{}", response)?;
        self.writer.flush()?;
        Ok(())
    }

    pub async fn handle_message(&mut self, message: &Value) -> Result<()> {
        let id = message.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let method = message.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let default_params = json!({});
        let params = message.get("params").unwrap_or(&default_params);

        match method {
            "ping" => self.handle_ping(id).await,
            "check_connection" => self.handle_check_connection(id).await,

            // UI Tree Tools
            "get_ui_tree" => self.handle_get_ui_tree(id).await,
            "find_by_label" => self.handle_find_by_label(id, params, false).await,
            "find_by_label_exact" => self.handle_find_by_label(id, params, true).await,
            "find_by_role" => self.handle_find_by_role(id, params).await,
            "get_element" => self.handle_get_element(id, params).await,

            // High Priority Element Interaction Tools
            "focus_element" => self.handle_focus_element(id, params).await,
            "get_element_value" => self.handle_get_element_value(id, params).await,
            "set_element_value" => self.handle_set_element_value(id, params).await,
            "type_text" => self.handle_type_text(id, params).await,
            "toggle_checkbox" => self.handle_toggle_checkbox(id, params).await,
            "set_checkbox" => self.handle_set_checkbox(id, params).await,
            "select_combobox_option" => self.handle_select_combobox_option(id, params).await,

            // Medium Priority Element Interaction Tools
            "get_checkbox_state" => self.handle_get_checkbox_state(id, params).await,
            "clear_text" => self.handle_clear_text(id, params).await,
            "get_selected_option" => self.handle_get_selected_option(id, params).await,
            "select_tab" => self.handle_select_tab(id, params).await,
            "get_active_tab" => self.handle_get_active_tab(id).await,
            "get_window_list" => self.handle_get_window_list(id).await,
            "wait_for_element" => self.handle_wait_for_element(id, params).await,

            // Debug & Logging Tools
            "get_element_path" => self.handle_get_element_path(id, params).await,
            "inspect_element_at_point" => self.handle_inspect_element_at_point(id, params).await,
            "get_supported_patterns" => self.handle_get_supported_patterns(id, params).await,
            "find_all_interactive_elements" => self.handle_find_all_interactive_elements(id).await,
            "get_element_debug_info" => self.handle_get_element_debug_info(id, params).await,
            "dump_ui_tree_detailed" => self.handle_dump_ui_tree_detailed(id, params).await,
            "get_element_siblings" => self.handle_get_element_siblings(id, params).await,

            // Quick Win Tools
            "get_center_point" => self.handle_get_center_point(id, params).await,
            "click_center" => self.handle_click_center(id, params).await,
            "find_nearest_element" => self.handle_find_nearest_element(id, params).await,
            "activate_element" => self.handle_activate_element(id, params).await,
            "get_clipboard" => self.handle_get_clipboard(id).await,
            "set_clipboard" => self.handle_set_clipboard(id, params).await,
            "scroll_to_element" => self.handle_scroll_to_element(id, params).await,
            "right_click_at" => self.handle_right_click_at(id, params).await,
            "right_click_element" => self.handle_right_click_element(id, params).await,
            "open_context_menu" => self.handle_open_context_menu(id, params).await,
            "select_menu_item" => self.handle_select_menu_item(id, params).await,
            "highlight_element" => self.handle_highlight_element(id, params).await,
            "flash_element" => self.handle_flash_element(id, params).await,
            "wait_for_element_stable" => self.handle_wait_for_element_stable(id, params).await,
            "wait_for_value_change" => self.handle_wait_for_value_change(id, params).await,

            // Coordinate-based Input Tools
            "click_at" => self.handle_click_at(id, params).await,
            "double_click" => self.handle_double_click(id, params).await,
            "hover" => self.handle_hover(id, params).await,
            "drag" => self.handle_drag(id, params).await,
            "keyboard_input" => self.handle_keyboard_input(id, params).await,
            "scroll" => self.handle_scroll(id, params).await,

            // Screenshot Tools
            "take_screenshot" => self.handle_take_screenshot(id).await,
            "screenshot_region" => self.handle_screenshot_region(id, params).await,

            // Convenience Tool
            "set_slider_value" => self.handle_set_slider_value(id, params).await,

            _ => {
                self.write_error(id, -32601, format!("Method not found: {}", method))?;
                Ok(())
            }
        }
    }

    // Handler: ping
    async fn handle_ping(&mut self, id: i64) -> Result<()> {
        self.write_response(id, json!({"status": "pong"}))
    }

    // Handler: check_connection
    async fn handle_check_connection(&mut self, id: i64) -> Result<()> {
        let ipc_connected = self.ipc_client.is_some();
        let uia_connected = self.uia_client.is_some();
        let uia_window_available = if let Some(uia) = &self.uia_client {
            uia.find_window_by_title(WINDOW_TITLE).await.is_ok()
        } else {
            false
        };

        self.write_response(id, json!({
            "ipc_connected": ipc_connected,
            "uia_connected": uia_connected,
            "uia_window_available": uia_window_available,
            "status": if ipc_connected && uia_window_available { "fully_connected" } else { "partial" }
        }))
    }

    // Handler: get_ui_tree
    async fn handle_get_ui_tree(&mut self, id: i64) -> Result<()> {
        if let Some(uia) = &self.uia_client {
            match uia.get_ui_tree(WINDOW_TITLE).await {
                Ok(tree) => self.write_response(id, tree),
                Err(e) => self.write_error(id, -32000, format!("Failed to get UI tree: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: find_by_label
    async fn handle_find_by_label(&mut self, id: i64, params: &Value, exact: bool) -> Result<()> {
        let label = match required_non_empty_string_param(params, "label") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };

        if let Some(uia) = &self.uia_client {
            match uia.find_elements_by_name(WINDOW_TITLE, label, exact).await {
                Ok(elements) => {
                    let result: Vec<Value> = elements
                        .iter()
                        .map(|e| {
                            json!({
                                "id": e.id,
                                "name": e.name,
                                "role": e.role,
                                "bounds": {
                                    "x": e.bounds.x,
                                    "y": e.bounds.y,
                                    "width": e.bounds.width,
                                    "height": e.bounds.height
                                }
                            })
                        })
                        .collect();
                    self.write_response(id, json!(result))
                }
                Err(e) => self.write_error(id, -32000, format!("Failed to find elements: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: find_by_role
    async fn handle_find_by_role(&mut self, id: i64, params: &Value) -> Result<()> {
        let role = match required_non_empty_string_param(params, "role") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };

        if let Some(uia) = &self.uia_client {
            match uia.find_elements_by_role(WINDOW_TITLE, role).await {
                Ok(elements) => {
                    let result: Vec<Value> = elements
                        .iter()
                        .map(|e| {
                            json!({
                                "id": e.id,
                                "name": e.name,
                                "role": e.role,
                                "bounds": {
                                    "x": e.bounds.x,
                                    "y": e.bounds.y,
                                    "width": e.bounds.width,
                                    "height": e.bounds.height
                                }
                            })
                        })
                        .collect();
                    self.write_response(id, json!(result))
                }
                Err(e) => self.write_error(id, -32000, format!("Failed to find elements: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_element
    async fn handle_get_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = match required_non_empty_string_param(params, "element_id") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };

        if let Some(uia) = &self.uia_client {
            match uia.get_element_by_id(WINDOW_TITLE, element_id).await {
                Ok(element) => self.write_response(
                    id,
                    json!({
                        "id": element.id,
                        "name": element.name,
                        "role": element.role,
                        "value": element.value,
                        "bounds": {
                            "x": element.bounds.x,
                            "y": element.bounds.y,
                            "width": element.bounds.width,
                            "height": element.bounds.height
                        },
                        "enabled": element.is_enabled,
                        "visible": element.is_visible,
                        "focused": element.has_focus
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Element not found: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: click_at
    async fn handle_click_at(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        if let Some(client) = self.ipc_client.as_mut() {
            match client.click_at(x, y).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: double_click
    async fn handle_double_click(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        if let Some(client) = self.ipc_client.as_mut() {
            match client.double_click(x, y).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: hover
    async fn handle_hover(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        if let Some(client) = self.ipc_client.as_mut() {
            match client.hover(x, y).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: drag
    async fn handle_drag(&mut self, id: i64, params: &Value) -> Result<()> {
        let x1 = params.get("x1").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y1 = params.get("y1").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let x2 = params.get("x2").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y2 = params.get("y2").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        if let Some(client) = self.ipc_client.as_mut() {
            match client.drag(x1, y1, x2, y2).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: keyboard_input
    async fn handle_keyboard_input(&mut self, id: i64, params: &Value) -> Result<()> {
        let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(client) = self.ipc_client.as_mut() {
            match client.keyboard_input(key).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: scroll
    async fn handle_scroll(&mut self, id: i64, params: &Value) -> Result<()> {
        let delta_x = params.get("delta_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let delta_y = params.get("delta_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        if let Some(client) = self.ipc_client.as_mut() {
            match client.scroll(delta_x, delta_y).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: take_screenshot
    async fn handle_take_screenshot(&mut self, id: i64) -> Result<()> {
        if let Some(client) = self.ipc_client.as_mut() {
            match client.take_screenshot().await {
                Ok(base64_image) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "image_base64": base64_image
                    }),
                ),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: screenshot_region
    async fn handle_screenshot_region(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let width = params.get("width").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
        let height = params.get("height").and_then(|v| v.as_i64()).unwrap_or(100) as i32;

        if let Some(client) = self.ipc_client.as_mut() {
            match client.screenshot_region(x, y, width, height).await {
                Ok(base64_image) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "image_base64": base64_image
                    }),
                ),
                Err(e) => self.write_error(id, -32000, e),
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: set_slider_value (convenience method)
    async fn handle_set_slider_value(&mut self, id: i64, params: &Value) -> Result<()> {
        let value = params.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

        if let Some(client) = self.ipc_client.as_mut() {
            // Find slider using UI Automation
            if let Some(uia) = &self.uia_client {
                match uia.find_elements_by_role(WINDOW_TITLE, "Slider").await {
                    Ok(sliders) => {
                        if let Some(slider) = sliders.first() {
                            let bounds = &slider.bounds;
                            let slider_x_start = bounds.x;
                            let slider_y = bounds.y + (bounds.height / 2);
                            let slider_width = bounds.width;

                            let target_x =
                                slider_x_start + ((value / 100.0) * slider_width as f64) as i32;

                            match client.click_at(slider_x_start, slider_y).await {
                                Ok(_) => {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(100))
                                        .await;
                                    match client.hover(target_x, slider_y).await {
                                        Ok(_) => self.write_response(
                                            id,
                                            json!({
                                                "success": true,
                                                "value": value,
                                                "message": format!("Slider set to {}", value)
                                            }),
                                        ),
                                        Err(e) => self.write_error(
                                            id,
                                            -32000,
                                            format!("Failed to drag slider: {}", e),
                                        ),
                                    }
                                }
                                Err(e) => self.write_error(
                                    id,
                                    -32000,
                                    format!("Failed to click slider: {}", e),
                                ),
                            }
                        } else {
                            self.write_error(id, -32000, "No slider found in UI".to_string())
                        }
                    }
                    Err(e) => self.write_error(id, -32000, format!("Failed to find slider: {}", e)),
                }
            } else {
                self.write_error(id, -32001, "UI Automation not available".to_string())
            }
        } else {
            self.write_error(id, -32001, "IPC client not connected".to_string())
        }
    }

    // Handler: focus_element
    async fn handle_focus_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.focus_element(WINDOW_TITLE, element_id).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, format!("Failed to focus element: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_element_value
    async fn handle_get_element_value(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_element_value(WINDOW_TITLE, element_id).await {
                Ok(value) => self.write_response(id, json!({"value": value})),
                Err(e) => self.write_error(id, -32000, format!("Failed to get value: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: set_element_value
    async fn handle_set_element_value(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let value = params.get("value").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.set_element_value(WINDOW_TITLE, element_id, value).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, format!("Failed to set value: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: type_text
    async fn handle_type_text(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.type_text(WINDOW_TITLE, element_id, text).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, format!("Failed to type text: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: toggle_checkbox
    async fn handle_toggle_checkbox(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.toggle_checkbox(WINDOW_TITLE, element_id).await {
                Ok(new_state) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "state": new_state
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Failed to toggle checkbox: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: set_checkbox
    async fn handle_set_checkbox(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let checked = params
            .get("checked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(uia) = &self.uia_client {
            match uia.set_checkbox(WINDOW_TITLE, element_id, checked).await {
                Ok(_) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "checked": checked
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Failed to set checkbox: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: select_combobox_option
    async fn handle_select_combobox_option(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let option_text = params
            .get("option_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia
                .select_combobox_option(WINDOW_TITLE, element_id, option_text)
                .await
            {
                Ok(_) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "selected": option_text
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Failed to select option: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_checkbox_state
    async fn handle_get_checkbox_state(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_checkbox_state(WINDOW_TITLE, element_id).await {
                Ok(state) => self.write_response(id, json!({"state": state})),
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to get checkbox state: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: clear_text
    async fn handle_clear_text(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.clear_text(WINDOW_TITLE, element_id).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, format!("Failed to clear text: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_selected_option
    async fn handle_get_selected_option(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_selected_option(WINDOW_TITLE, element_id).await {
                Ok(option) => self.write_response(id, json!({"option": option})),
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to get selected option: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: select_tab
    async fn handle_select_tab(&mut self, id: i64, params: &Value) -> Result<()> {
        let tab_name = params
            .get("tab_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.select_tab(WINDOW_TITLE, tab_name).await {
                Ok(_) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "tab": tab_name
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Failed to select tab: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_active_tab
    async fn handle_get_active_tab(&mut self, id: i64) -> Result<()> {
        if let Some(uia) = &self.uia_client {
            match uia.get_active_tab(WINDOW_TITLE).await {
                Ok(tab_name) => self.write_response(id, json!({"tab": tab_name})),
                Err(e) => self.write_error(id, -32000, format!("Failed to get active tab: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_window_list
    async fn handle_get_window_list(&mut self, id: i64) -> Result<()> {
        if let Some(uia) = &self.uia_client {
            match uia.get_window_list().await {
                Ok(windows) => self.write_response(id, json!({"windows": windows})),
                Err(e) => self.write_error(id, -32000, format!("Failed to get window list: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: wait_for_element
    async fn handle_wait_for_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(5000);

        if let Some(uia) = &self.uia_client {
            match uia
                .wait_for_element(WINDOW_TITLE, element_id, timeout_ms)
                .await
            {
                Ok(element) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "element": {
                            "id": element.id,
                            "name": element.name,
                            "role": element.role,
                            "bounds": {
                                "x": element.bounds.x,
                                "y": element.bounds.y,
                                "width": element.bounds.width,
                                "height": element.bounds.height
                            }
                        }
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Wait for element failed: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_element_path
    async fn handle_get_element_path(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_element_path(WINDOW_TITLE, element_id).await {
                Ok(path) => self.write_response(id, json!({"path": path})),
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to get element path: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: inspect_element_at_point
    async fn handle_inspect_element_at_point(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        if let Some(uia) = &self.uia_client {
            match uia.inspect_element_at_point(x, y).await {
                Ok(element) => self.write_response(
                    id,
                    json!({
                        "element": {
                            "id": element.id,
                            "name": element.name,
                            "role": element.role,
                            "value": element.value,
                            "bounds": {
                                "x": element.bounds.x,
                                "y": element.bounds.y,
                                "width": element.bounds.width,
                                "height": element.bounds.height
                            },
                            "enabled": element.is_enabled,
                            "visible": element.is_visible,
                            "focused": element.has_focus
                        }
                    }),
                ),
                Err(e) => self.write_error(id, -32000, format!("Failed to inspect element: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_supported_patterns
    async fn handle_get_supported_patterns(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_supported_patterns(WINDOW_TITLE, element_id).await {
                Ok(patterns) => self.write_response(id, json!({"patterns": patterns})),
                Err(e) => self.write_error(id, -32000, format!("Failed to get patterns: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: find_all_interactive_elements
    async fn handle_find_all_interactive_elements(&mut self, id: i64) -> Result<()> {
        if let Some(uia) = &self.uia_client {
            match uia.find_all_interactive_elements(WINDOW_TITLE).await {
                Ok(elements) => {
                    let elements_json: Vec<Value> = elements
                        .iter()
                        .map(|e| {
                            json!({
                                "id": e.id,
                                "name": e.name,
                                "role": e.role,
                                "value": e.value,
                                "bounds": {
                                    "x": e.bounds.x,
                                    "y": e.bounds.y,
                                    "width": e.bounds.width,
                                    "height": e.bounds.height
                                },
                                "enabled": e.is_enabled,
                                "visible": e.is_visible
                            })
                        })
                        .collect();

                    self.write_response(
                        id,
                        json!({
                            "elements": elements_json,
                            "count": elements.len()
                        }),
                    )
                }
                Err(e) => self.write_error(
                    id,
                    -32000,
                    format!("Failed to find interactive elements: {}", e),
                ),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_element_debug_info
    async fn handle_get_element_debug_info(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_element_debug_info(WINDOW_TITLE, element_id).await {
                Ok(debug_info) => self.write_response(id, debug_info),
                Err(e) => self.write_error(id, -32000, format!("Failed to get debug info: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: dump_ui_tree_detailed
    async fn handle_dump_ui_tree_detailed(&mut self, id: i64, params: &Value) -> Result<()> {
        let max_depth = params
            .get("max_depth")
            .and_then(|v| v.as_i64())
            .unwrap_or(5) as i32;

        if let Some(uia) = &self.uia_client {
            match uia.dump_ui_tree_detailed(WINDOW_TITLE, max_depth).await {
                Ok(tree) => self.write_response(id, tree),
                Err(e) => self.write_error(id, -32000, format!("Failed to dump UI tree: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_element_siblings
    async fn handle_get_element_siblings(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_element_siblings(WINDOW_TITLE, element_id).await {
                Ok(siblings) => {
                    let siblings_json: Vec<Value> = siblings
                        .iter()
                        .map(|e| {
                            json!({
                                "id": e.id,
                                "name": e.name,
                                "role": e.role,
                                "bounds": {
                                    "x": e.bounds.x,
                                    "y": e.bounds.y,
                                    "width": e.bounds.width,
                                    "height": e.bounds.height
                                }
                            })
                        })
                        .collect();

                    self.write_response(
                        id,
                        json!({
                            "siblings": siblings_json,
                            "count": siblings.len()
                        }),
                    )
                }
                Err(e) => self.write_error(id, -32000, format!("Failed to get siblings: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_center_point
    async fn handle_get_center_point(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.get_center_point(WINDOW_TITLE, element_id).await {
                Ok((x, y)) => self.write_response(id, json!({"x": x, "y": y})),
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to get center point: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: click_center
    async fn handle_click_center(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.click_center(WINDOW_TITLE, element_id).await {
                Ok((x, y)) => {
                    if let Some(ipc) = &mut self.ipc_client {
                        match ipc.click_at(x, y).await {
                            Ok(_) => self.write_response(
                                id,
                                json!({
                                    "success": true,
                                    "x": x,
                                    "y": y
                                }),
                            ),
                            Err(e) => {
                                self.write_error(id, -32000, format!("Failed to click: {}", e))
                            }
                        }
                    } else {
                        self.write_error(id, -32001, "IPC client not connected".to_string())
                    }
                }
                Err(e) => self.write_error(id, -32000, format!("Failed to get center: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: find_nearest_element
    async fn handle_find_nearest_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = match params.get("x").and_then(|v| v.as_i64()) {
            Some(value) => value as i32,
            None => {
                return self.write_error(id, -32602, "Missing required parameter 'x'".to_string())
            }
        };
        let y = match params.get("y").and_then(|v| v.as_i64()) {
            Some(value) => value as i32,
            None => {
                return self.write_error(id, -32602, "Missing required parameter 'y'".to_string())
            }
        };
        let max_distance = params.get("max_distance").and_then(|v| v.as_f64());

        if let Some(uia) = &self.uia_client {
            match uia
                .find_nearest_interactive_element(WINDOW_TITLE, x, y, max_distance)
                .await
            {
                Ok((element, distance)) => {
                    let center_x = element.bounds.x + (element.bounds.width / 2);
                    let center_y = element.bounds.y + (element.bounds.height / 2);
                    self.write_response(
                        id,
                        json!({
                            "element": {
                                "id": element.id,
                                "name": element.name,
                                "role": element.role,
                                "bounds": {
                                    "x": element.bounds.x,
                                    "y": element.bounds.y,
                                    "width": element.bounds.width,
                                    "height": element.bounds.height
                                }
                            },
                            "center": { "x": center_x, "y": center_y },
                            "distance": distance
                        }),
                    )
                }
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to find nearest element: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: activate_element
    async fn handle_activate_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.activate_element(WINDOW_TITLE, element_id).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to activate element: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: get_clipboard
    async fn handle_get_clipboard(&mut self, id: i64) -> Result<()> {
        if let Some(uia) = &self.uia_client {
            match uia.get_clipboard().await {
                Ok(text) => self.write_response(id, json!({"text": text})),
                Err(e) => self.write_error(id, -32000, format!("Failed to get clipboard: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: set_clipboard
    async fn handle_set_clipboard(&mut self, id: i64, params: &Value) -> Result<()> {
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.set_clipboard(text).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => self.write_error(id, -32000, format!("Failed to set clipboard: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: scroll_to_element
    async fn handle_scroll_to_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = params
            .get("element_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Some(uia) = &self.uia_client {
            match uia.scroll_to_element(WINDOW_TITLE, element_id).await {
                Ok(_) => self.write_response(id, json!({"success": true})),
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to scroll to element: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: right_click_at
    async fn handle_right_click_at(&mut self, id: i64, params: &Value) -> Result<()> {
        let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        // Call input module directly (not through IPC)
        match egui_mcp_client_win::input::send_right_click(x, y) {
            Ok(_) => self.write_response(id, json!({"success": true})),
            Err(e) => self.write_error(id, -32000, format!("Failed to right click: {}", e)),
        }
    }

    // Handler: right_click_element
    async fn handle_right_click_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = match required_non_empty_string_param(params, "element_id") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };

        if let Some(uia) = &self.uia_client {
            match uia.get_center_point(WINDOW_TITLE, element_id).await {
                Ok((x, y)) => match egui_mcp_client_win::input::send_right_click(x, y) {
                    Ok(_) => self.write_response(id, json!({"success": true, "x": x, "y": y})),
                    Err(e) => self.write_error(
                        id,
                        -32000,
                        format!("Failed to right click element center: {}", e),
                    ),
                },
                Err(e) => self.write_error(
                    id,
                    -32000,
                    format!("Failed to resolve element center: {}", e),
                ),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: open_context_menu
    async fn handle_open_context_menu(&mut self, id: i64, params: &Value) -> Result<()> {
        self.handle_right_click_element(id, params).await
    }

    // Handler: select_menu_item
    async fn handle_select_menu_item(&mut self, id: i64, params: &Value) -> Result<()> {
        let label = match required_non_empty_string_param(params, "label") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };
        let exact = params
            .get("exact")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if let Some(uia) = &self.uia_client {
            match uia.find_elements_by_name(WINDOW_TITLE, label, exact).await {
                Ok(elements) => {
                    let menu_item = elements
                        .iter()
                        .find(|element| element.role.eq_ignore_ascii_case("MenuItem"));
                    let fallback = elements.first();
                    let Some(target) = menu_item.or(fallback) else {
                        return self.write_error(
                            id,
                            -32000,
                            format!("No element found matching menu label '{}'", label),
                        );
                    };

                    match uia.activate_element(WINDOW_TITLE, &target.id).await {
                        Ok(_) => self.write_response(
                            id,
                            json!({
                                "success": true,
                                "item": {
                                    "id": target.id,
                                    "name": target.name,
                                    "role": target.role
                                },
                                "used_menu_role": menu_item.is_some()
                            }),
                        ),
                        Err(e) => self.write_error(
                            id,
                            -32000,
                            format!("Failed to activate menu item: {}", e),
                        ),
                    }
                }
                Err(e) => {
                    self.write_error(id, -32000, format!("Failed to search menu items: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: highlight_element (single quick flash pass)
    async fn handle_highlight_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = match required_non_empty_string_param(params, "element_id") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };
        let delay_ms = params
            .get("delay_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(40);

        if let Some(uia) = &self.uia_client {
            match uia.get_element_by_id(WINDOW_TITLE, element_id).await {
                Ok(element) => {
                    let points = flash_points_for_rect(&element.bounds);
                    for (x, y) in points {
                        if let Err(e) = egui_mcp_client_win::input::send_mouse_move(x, y) {
                            return self.write_error(
                                id,
                                -32000,
                                format!("Failed to highlight element: {}", e),
                            );
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }

                    self.write_response(
                        id,
                        json!({
                            "success": true,
                            "element_id": element.id,
                            "bounds": {
                                "x": element.bounds.x,
                                "y": element.bounds.y,
                                "width": element.bounds.width,
                                "height": element.bounds.height
                            }
                        }),
                    )
                }
                Err(e) => self.write_error(id, -32000, format!("Failed to get element: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: flash_element (multi-pass visual flash)
    async fn handle_flash_element(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = match required_non_empty_string_param(params, "element_id") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };
        let flashes = params.get("flashes").and_then(|v| v.as_u64()).unwrap_or(3);
        let delay_ms = params
            .get("delay_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);

        if let Some(uia) = &self.uia_client {
            match uia.get_element_by_id(WINDOW_TITLE, element_id).await {
                Ok(element) => {
                    let points = flash_points_for_rect(&element.bounds);
                    for _ in 0..flashes.max(1) {
                        for (x, y) in &points {
                            if let Err(e) = egui_mcp_client_win::input::send_mouse_move(*x, *y) {
                                return self.write_error(
                                    id,
                                    -32000,
                                    format!("Failed to flash element: {}", e),
                                );
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        }
                    }

                    self.write_response(
                        id,
                        json!({
                            "success": true,
                            "element_id": element.id,
                            "flashes": flashes.max(1)
                        }),
                    )
                }
                Err(e) => self.write_error(id, -32000, format!("Failed to get element: {}", e)),
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: wait_for_element_stable
    async fn handle_wait_for_element_stable(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = match required_non_empty_string_param(params, "element_id") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };
        let stable_ms = params
            .get("stable_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(5000);
        let poll_interval_ms = params
            .get("poll_interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);

        if let Some(uia) = &self.uia_client {
            match uia
                .wait_for_element_stable(
                    WINDOW_TITLE,
                    element_id,
                    stable_ms,
                    timeout_ms,
                    poll_interval_ms,
                )
                .await
            {
                Ok(element) => self.write_response(
                    id,
                    json!({
                        "success": true,
                        "element": {
                            "id": element.id,
                            "name": element.name,
                            "role": element.role,
                            "bounds": {
                                "x": element.bounds.x,
                                "y": element.bounds.y,
                                "width": element.bounds.width,
                                "height": element.bounds.height
                            }
                        }
                    }),
                ),
                Err(e) => {
                    self.write_error(id, -32000, format!("wait_for_element_stable failed: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }

    // Handler: wait_for_value_change
    async fn handle_wait_for_value_change(&mut self, id: i64, params: &Value) -> Result<()> {
        let element_id = match required_non_empty_string_param(params, "element_id") {
            Ok(value) => value,
            Err(message) => return self.write_error(id, -32602, message),
        };
        let initial_value = params.get("initial_value").and_then(|v| v.as_str());
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(5000);
        let poll_interval_ms = params
            .get("poll_interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);

        if let Some(uia) = &self.uia_client {
            match uia
                .wait_for_value_change(
                    WINDOW_TITLE,
                    element_id,
                    initial_value,
                    timeout_ms,
                    poll_interval_ms,
                )
                .await
            {
                Ok(value) => self.write_response(id, json!({ "success": true, "value": value })),
                Err(e) => {
                    self.write_error(id, -32000, format!("wait_for_value_change failed: {}", e))
                }
            }
        } else {
            self.write_error(id, -32001, "UI Automation not available".to_string())
        }
    }
}

pub struct EguiMcpServer;

impl EguiMcpServer {
    pub async fn new() -> Result<Self> {
        info!("Initializing egui-mcp-server");
        Ok(Self)
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting MCP server...");

        let stdin = io::stdin();
        let stdout = io::stdout();

        let reader = io::BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        let mut server = McpServer::new(reader, &mut writer);

        // Connect to IPC and UI Automation
        if let Err(e) = server.connect().await {
            error!("Failed during connection setup: {}", e);
        }

        info!("Server ready, waiting for MCP messages...");

        loop {
            if let Some(message) = server.read_message()? {
                info!("Received message: {:?}", message);
                server.handle_message(&message).await?;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_non_empty_string_param_accepts_trimmed_string() {
        let params = json!({ "label": "  Checkbox  " });
        let value = required_non_empty_string_param(&params, "label").unwrap();
        assert_eq!(value, "Checkbox");
    }

    #[test]
    fn required_non_empty_string_param_rejects_missing_or_empty() {
        let missing = json!({});
        assert!(required_non_empty_string_param(&missing, "label").is_err());

        let empty = json!({ "label": "   " });
        assert!(required_non_empty_string_param(&empty, "label").is_err());
    }

    #[test]
    fn required_non_empty_string_param_rejects_non_string() {
        let params = json!({ "label": 42 });
        assert!(required_non_empty_string_param(&params, "label").is_err());
    }

    #[test]
    fn flash_points_for_rect_returns_center_and_corners() {
        let rect = UiaRect {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        };

        let points = flash_points_for_rect(&rect);
        assert_eq!(points.len(), 5);
        assert_eq!(points[0], (10, 20));
        assert_eq!(points[4], (25, 40));
    }
}
