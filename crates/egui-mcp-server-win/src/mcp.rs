//! MCP protocol surface: tool discovery schemas and result envelopes.
//!
//! The server keeps its bare JSON-RPC method names as the internal dispatch
//! vocabulary. This module is the translation layer that lets a standard MCP
//! client discover those methods via `tools/list` and invoke them through
//! `tools/call`.

use serde_json::{json, Value};

/// Image fields that are lifted out of a tool result into an `image` content
/// block rather than being inlined as base64 text.
const IMAGE_FIELDS: &[&str] = &["image_base64", "diff_image_base64"];

fn string_prop(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn integer_prop(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

fn number_prop(description: &str) -> Value {
    json!({ "type": "number", "description": description })
}

fn boolean_prop(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

fn element_id_prop() -> Value {
    string_prop("UI Automation identifier of the target element, as returned by the find_* or get_ui_tree tools.")
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required
        }
    })
}

/// A tool taking only `element_id`.
fn element_tool(name: &str, description: &str) -> Value {
    tool(
        name,
        description,
        json!({ "element_id": element_id_prop() }),
        &["element_id"],
    )
}

/// A tool taking only screen coordinates.
fn point_tool(name: &str, description: &str) -> Value {
    tool(
        name,
        description,
        json!({
            "x": integer_prop("Screen X coordinate in physical pixels."),
            "y": integer_prop("Screen Y coordinate in physical pixels.")
        }),
        &["x", "y"],
    )
}

/// A tool taking no arguments.
fn no_arg_tool(name: &str, description: &str) -> Value {
    tool(name, description, json!({}), &[])
}

/// Every tool this server exposes, in the order clients should see them.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        // --- Connection and configuration ---
        no_arg_tool("ping", "Check that the MCP server process is responsive."),
        no_arg_tool(
            "check_connection",
            "Report IPC and UI Automation connection status, plus whether the configured target window is currently visible to UIA.",
        ),
        no_arg_tool(
            "get_target_window",
            "Return the window title the server currently targets for all UI Automation tools.",
        ),
        tool(
            "set_target_window",
            "Point the server at a different window title. Matching is case-insensitive and accepts partial titles. Use this when automating an app other than the default.",
            json!({
                "title": string_prop("Window title to target, full or partial, case-insensitive.")
            }),
            &["title"],
        ),

        // --- Tree and queries ---
        no_arg_tool(
            "get_ui_tree",
            "Return the full accessibility tree of the target window as nested JSON.",
        ),
        tool(
            "find_by_label",
            "Find elements whose accessible name contains the given text (case-insensitive substring match).",
            json!({ "label": string_prop("Text to search for within element names.") }),
            &["label"],
        ),
        tool(
            "find_by_label_exact",
            "Find elements whose accessible name exactly equals the given text.",
            json!({ "label": string_prop("Exact element name to match.") }),
            &["label"],
        ),
        tool(
            "find_by_role",
            "Find elements by UI Automation control type, for example Button, CheckBox, Edit, Slider, or TabItem.",
            json!({ "role": string_prop("UIA control type name, for example 'Button' or 'CheckBox'.") }),
            &["role"],
        ),
        element_tool(
            "get_element",
            "Fetch a single element's name, role, value, bounds, and state by its identifier.",
        ),
        element_tool(
            "get_element_path",
            "Return the ancestor chain from the window root down to the given element.",
        ),
        element_tool(
            "get_element_siblings",
            "List the elements sharing a parent with the given element.",
        ),
        element_tool(
            "get_supported_patterns",
            "List the UI Automation control patterns the given element supports.",
        ),
        element_tool(
            "get_element_debug_info",
            "Return verbose diagnostic detail for one element, including raw UIA property values.",
        ),
        no_arg_tool(
            "find_all_interactive_elements",
            "List every actionable element in the target window (buttons, inputs, checkboxes, and similar).",
        ),
        tool(
            "dump_ui_tree_detailed",
            "Dump the accessibility tree with full per-element properties, optionally limited by depth.",
            json!({ "max_depth": integer_prop("Maximum tree depth to walk. Omit for unlimited.") }),
            &[],
        ),
        point_tool(
            "inspect_element_at_point",
            "Identify the element located at the given screen coordinates.",
        ),
        no_arg_tool(
            "get_window_list",
            "List all top-level windows currently visible to UI Automation. Useful for discovering a title to pass to set_target_window.",
        ),

        // --- Element interaction ---
        element_tool("focus_element", "Move keyboard focus to the given element."),
        element_tool(
            "activate_element",
            "Invoke the given element's default action, equivalent to clicking it.",
        ),
        element_tool(
            "click_center",
            "Click the geometric center of the given element.",
        ),
        element_tool(
            "get_center_point",
            "Return the screen coordinates of the given element's center without clicking it.",
        ),
        element_tool(
            "scroll_to_element",
            "Scroll the given element into view.",
        ),
        element_tool(
            "get_element_value",
            "Read the given element's value via the UIA Value pattern.",
        ),
        tool(
            "set_element_value",
            "Set the given element's value via the UIA Value pattern.",
            json!({
                "element_id": element_id_prop(),
                "value": string_prop("New value to write.")
            }),
            &["element_id", "value"],
        ),
        tool(
            "type_text",
            "Focus the given element and enter text into it in one call. Prefer this over set_element_value for egui text fields.",
            json!({
                "element_id": element_id_prop(),
                "text": string_prop("Text to enter.")
            }),
            &["element_id", "text"],
        ),
        element_tool("clear_text", "Clear the text content of the given element."),
        element_tool(
            "toggle_checkbox",
            "Flip the given checkbox and return its resulting state.",
        ),
        tool(
            "set_checkbox",
            "Drive the given checkbox to an explicit checked or unchecked state.",
            json!({
                "element_id": element_id_prop(),
                "checked": boolean_prop("Desired state: true for checked, false for unchecked.")
            }),
            &["element_id", "checked"],
        ),
        element_tool(
            "get_checkbox_state",
            "Read whether the given checkbox is currently checked.",
        ),
        tool(
            "select_combobox_option",
            "Expand the given combo box and select an option by its visible text.",
            json!({
                "element_id": element_id_prop(),
                "option_text": string_prop("Visible text of the option to select.")
            }),
            &["element_id", "option_text"],
        ),
        element_tool(
            "get_selected_option",
            "Read the currently selected option of the given combo box.",
        ),
        tool(
            "select_tab",
            "Activate a tab by its visible name.",
            json!({ "tab_name": string_prop("Visible name of the tab to activate.") }),
            &["tab_name"],
        ),
        no_arg_tool("get_active_tab", "Return the currently active tab."),
        tool(
            "set_slider_value",
            "Set the first slider in the target window to a percentage of its track.",
            json!({ "value": number_prop("Target position as a percentage from 0 to 100.") }),
            &["value"],
        ),

        // --- Context menus ---
        element_tool(
            "right_click_element",
            "Right-click the given element.",
        ),
        element_tool(
            "open_context_menu",
            "Open the context menu for the given element.",
        ),
        tool(
            "select_menu_item",
            "Select an open menu item by its label.",
            json!({
                "label": string_prop("Label of the menu item to select."),
                "exact": boolean_prop("Require an exact label match instead of a substring match. Defaults to false.")
            }),
            &["label"],
        ),

        // --- Waiting ---
        tool(
            "wait_for_element",
            "Block until the given element appears, or until the timeout elapses.",
            json!({
                "element_id": element_id_prop(),
                "timeout_ms": integer_prop("Maximum time to wait, in milliseconds.")
            }),
            &["element_id"],
        ),
        tool(
            "wait_for_element_stable",
            "Block until the given element's bounds stop changing for a sustained period.",
            json!({
                "element_id": element_id_prop(),
                "stable_ms": integer_prop("How long the element must remain unchanged, in milliseconds."),
                "timeout_ms": integer_prop("Maximum total time to wait, in milliseconds."),
                "poll_interval_ms": integer_prop("How often to re-check, in milliseconds.")
            }),
            &["element_id"],
        ),
        tool(
            "wait_for_value_change",
            "Block until the given element's value differs from a known starting value.",
            json!({
                "element_id": element_id_prop(),
                "initial_value": string_prop("Value to wait for the element to move away from."),
                "timeout_ms": integer_prop("Maximum time to wait, in milliseconds."),
                "poll_interval_ms": integer_prop("How often to re-check, in milliseconds.")
            }),
            &["element_id"],
        ),

        // --- Spatial ---
        tool(
            "find_nearest_element",
            "Find the interactive element closest to the given screen coordinates.",
            json!({
                "x": integer_prop("Screen X coordinate in physical pixels."),
                "y": integer_prop("Screen Y coordinate in physical pixels."),
                "max_distance": integer_prop("Ignore elements farther than this many pixels.")
            }),
            &["x", "y"],
        ),

        // --- Visual debugging ---
        tool(
            "highlight_element",
            "Trace the cursor around the given element's bounds to make it visible on screen.",
            json!({
                "element_id": element_id_prop(),
                "delay_ms": integer_prop("Pause between cursor moves, in milliseconds.")
            }),
            &["element_id"],
        ),
        tool(
            "flash_element",
            "Repeatedly trace the given element's bounds to draw attention to it.",
            json!({
                "element_id": element_id_prop(),
                "flashes": integer_prop("Number of times to repeat the trace."),
                "delay_ms": integer_prop("Pause between cursor moves, in milliseconds.")
            }),
            &["element_id"],
        ),

        // --- Raw input ---
        point_tool("click_at", "Left-click at the given screen coordinates."),
        point_tool("double_click", "Double-click at the given screen coordinates."),
        point_tool("right_click_at", "Right-click at the given screen coordinates."),
        point_tool("hover", "Move the cursor to the given screen coordinates."),
        tool(
            "drag",
            "Drag the cursor from one screen coordinate to another.",
            json!({
                "x1": integer_prop("Starting screen X coordinate."),
                "y1": integer_prop("Starting screen Y coordinate."),
                "x2": integer_prop("Ending screen X coordinate."),
                "y2": integer_prop("Ending screen Y coordinate.")
            }),
            &["x1", "y1", "x2", "y2"],
        ),
        tool(
            "keyboard_input",
            "Press and release a single key.",
            json!({ "key": string_prop("Key name to press, for example 'Enter', 'Tab', or 'a'.") }),
            &["key"],
        ),
        tool(
            "scroll",
            "Scroll by the given wheel deltas.",
            json!({
                "delta_x": integer_prop("Horizontal scroll amount."),
                "delta_y": integer_prop("Vertical scroll amount.")
            }),
            &[],
        ),

        // --- Clipboard ---
        no_arg_tool("get_clipboard", "Read the current clipboard text."),
        tool(
            "set_clipboard",
            "Replace the clipboard contents with the given text.",
            json!({ "text": string_prop("Text to place on the clipboard.") }),
            &["text"],
        ),

        // --- Screenshots ---
        no_arg_tool(
            "take_screenshot",
            "Capture the full target window as a PNG image.",
        ),
        tool(
            "screenshot_region",
            "Capture a rectangular screen region as a PNG image.",
            json!({
                "x": integer_prop("Left edge of the region in physical pixels."),
                "y": integer_prop("Top edge of the region in physical pixels."),
                "width": integer_prop("Region width in physical pixels."),
                "height": integer_prop("Region height in physical pixels.")
            }),
            &["x", "y", "width", "height"],
        ),
        element_tool(
            "screenshot_element",
            "Capture just the given element's bounds as a PNG image.",
        ),
        tool(
            "compare_screenshots",
            "Compare two base64 PNG images and report their similarity.",
            json!({
                "image_a_base64": string_prop("First image as base64-encoded PNG."),
                "image_b_base64": string_prop("Second image as base64-encoded PNG."),
                "threshold": number_prop("Similarity ratio from 0.0 to 1.0 required to count as a match. Defaults to 0.99.")
            }),
            &["image_a_base64", "image_b_base64"],
        ),
        tool(
            "diff_screenshots",
            "Compare two base64 PNG images and return a visual diff highlighting changed pixels.",
            json!({
                "image_a_base64": string_prop("First image as base64-encoded PNG."),
                "image_b_base64": string_prop("Second image as base64-encoded PNG."),
                "sensitivity": number_prop("Per-pixel difference from 0.0 to 1.0 that counts as a change. Defaults to 0.05."),
                "highlight_color": string_prop("Hex color used to mark changed pixels, for example '#FF0000'.")
            }),
            &["image_a_base64", "image_b_base64"],
        ),

        // --- Snapshots ---
        tool(
            "save_snapshot",
            "Store a screenshot for later comparison. Captures the target window if no image is supplied.",
            json!({
                "image_base64": string_prop("Image to store as base64-encoded PNG. Omit to capture the target window now."),
                "name": string_prop("Stable name to store the snapshot under, for reuse across runs."),
                "label": string_prop("Free-form label describing the snapshot.")
            }),
            &[],
        ),
        tool(
            "load_snapshot",
            "Retrieve a stored snapshot by identifier or by name. Supply exactly one of the two.",
            json!({
                "snapshot_id": string_prop("Identifier returned by save_snapshot."),
                "name": string_prop("Name the snapshot was stored under.")
            }),
            &[],
        ),
        tool(
            "diff_snapshots",
            "Compare two stored snapshots and return a visual diff. Identify each side by id or by name.",
            json!({
                "left_snapshot_id": string_prop("Identifier of the left snapshot."),
                "left_name": string_prop("Name of the left snapshot."),
                "right_snapshot_id": string_prop("Identifier of the right snapshot."),
                "right_name": string_prop("Name of the right snapshot."),
                "threshold": number_prop("Similarity ratio from 0.0 to 1.0 required to count as a match."),
                "sensitivity": number_prop("Per-pixel difference from 0.0 to 1.0 that counts as a change."),
                "highlight_color": string_prop("Hex color used to mark changed pixels, for example '#FF0000'.")
            }),
            &[],
        ),
    ]
}

/// Converts a handler result into MCP `content` blocks.
///
/// Base64 image fields become dedicated `image` blocks so clients can render
/// them, and are stripped from the accompanying JSON text so a multi-megabyte
/// blob is not repeated twice in one response.
pub fn result_to_content(result: &Value) -> Vec<Value> {
    let mut content = Vec::new();
    let mut remainder = result.clone();

    if let Some(object) = remainder.as_object_mut() {
        for field in IMAGE_FIELDS {
            let Some(encoded) = object.get(*field).and_then(Value::as_str) else {
                continue;
            };
            let encoded = encoded.to_string();
            if encoded.is_empty() {
                continue;
            }

            content.push(json!({
                "type": "image",
                "data": encoded,
                "mimeType": "image/png"
            }));
            object.insert(
                (*field).to_string(),
                Value::String(format!("<returned as {} image content>", field)),
            );
        }
    }

    let text = serde_json::to_string_pretty(&remainder)
        .unwrap_or_else(|_| remainder.to_string());
    content.push(json!({ "type": "text", "text": text }));

    content
}

/// Names of every advertised tool, used to keep discovery and dispatch aligned.
#[cfg(test)]
pub fn tool_names() -> Vec<String> {
    tool_definitions()
        .into_iter()
        .filter_map(|tool| {
            tool.get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;

    #[test]
    fn every_tool_has_a_name_description_and_object_schema() {
        for definition in tool_definitions() {
            let name = definition
                .get("name")
                .and_then(Value::as_str)
                .expect("tool missing name");
            assert!(!name.is_empty(), "tool name must not be empty");

            let description = definition
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("tool '{}' missing description", name));
            assert!(
                !description.is_empty(),
                "tool '{}' has an empty description",
                name
            );

            let schema = definition
                .get("inputSchema")
                .unwrap_or_else(|| panic!("tool '{}' missing inputSchema", name));
            assert_eq!(
                schema.get("type").and_then(Value::as_str),
                Some("object"),
                "tool '{}' schema must be an object",
                name
            );
            assert!(
                schema.get("properties").map(Value::is_object) == Some(true),
                "tool '{}' schema must declare properties",
                name
            );
        }
    }

    #[test]
    fn required_fields_are_declared_as_properties() {
        for definition in tool_definitions() {
            let name = definition["name"].as_str().unwrap();
            let schema = &definition["inputSchema"];
            let properties = schema["properties"].as_object().unwrap();
            let required = schema["required"].as_array().unwrap();

            for entry in required {
                let key = entry.as_str().unwrap();
                assert!(
                    properties.contains_key(key),
                    "tool '{}' requires '{}' but does not declare it as a property",
                    name,
                    key
                );
            }
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let names = tool_names();
        let mut seen: Map<String, Value> = Map::new();
        for name in names {
            assert!(
                seen.insert(name.clone(), Value::Null).is_none(),
                "duplicate tool name '{}'",
                name
            );
        }
    }

    #[test]
    fn image_fields_become_image_content_blocks() {
        let result = json!({ "success": true, "image_base64": "QUJD" });
        let content = result_to_content(&result);

        assert_eq!(content.len(), 2, "expected one image block and one text block");
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["data"], "QUJD");
        assert_eq!(content[0]["mimeType"], "image/png");

        let text = content[1]["text"].as_str().unwrap();
        assert!(
            !text.contains("QUJD"),
            "base64 payload should be stripped from the text block"
        );
    }

    #[test]
    fn results_without_images_produce_a_single_text_block() {
        let content = result_to_content(&json!({ "status": "pong" }));

        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"].as_str().unwrap().contains("pong"));
    }
}
