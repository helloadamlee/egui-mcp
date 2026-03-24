# egui-mcp Windows - Complete API Reference

## 📚 Overview

This document provides a complete reference for all MCP tools available in the Windows port of egui-mcp.

**Total Tools**: 21+ tools across multiple categories

**Availability note**: This file is API shape/reference. For current implementation and remaining work, see `ROADMAP_REMAINING.md`.

---

## 🔍 UI Discovery Tools

### `get_ui_tree`
Get the complete UI element hierarchy as JSON.

**Parameters**: None  
**Returns**:
```json
{
  "name": "Window Title",
  "role": "Window",
  "automation_id": "main_window",
  "children": [...]
}
```

**Example**:
```json
{"jsonrpc": "2.0", "id": 1, "method": "get_ui_tree"}
```

---

### `find_by_label`
Find elements by display name (partial match).

**Parameters**:
- `label` (string, required, non-empty): Text to search for in element names

**Returns**: Array of matching elements
```json
[
  {
    "id": "button_1",
    "name": "Save",
    "role": "Button",
    "bounds": {"x": 100, "y": 50, "width": 80, "height": 30}
  }
]
```

**Errors**:
- `-32602` when `label` is missing, not a string, or empty

---

### `find_by_label_exact`
Find elements by exact display name.

**Parameters**:
- `label` (string, required, non-empty): Exact text to match

**Errors**:
- `-32602` when `label` is missing, not a string, or empty

---

### `find_by_role`
Find all elements of a specific control type.

**Parameters**:
- `role` (string, required, non-empty): Control type (e.g., "Button", "Edit", "Slider", "CheckBox")

**Supported Roles**: Button, Calendar, CheckBox, ComboBox, Edit, Hyperlink, Image, List, ListItem, Menu, MenuBar, MenuItem, ProgressBar, RadioButton, ScrollBar, Slider, Spinner, StatusBar, Tab, TabItem, Text, ToolBar, ToolTip, Tree, TreeItem, DataGrid, Document, Window, Pane, Table, Group

**Errors**:
- `-32602` when `role` is missing, not a string, or empty
- `-32000` when the role value is unsupported

---

### `get_element`
Get detailed information about a specific element by ID.

**Parameters**:
- `element_id` (string, required, non-empty): Automation ID of the element

**Returns**:
```json
{
  "id": "text_input_1",
  "name": "Username",
  "role": "Edit",
  "value": "john_doe",
  "bounds": {"x": 50, "y": 100, "width": 200, "height": 25},
  "enabled": true,
  "visible": true,
  "focused": false
}
```

**Errors**:
- `-32602` when `element_id` is missing, not a string, or empty
- `-32000` when no element exists for the provided automation ID

---

## 🎯 High Priority Interaction Tools

### `focus_element`
Set keyboard focus to a specific element.

**Parameters**:
- `element_id` (string): Automation ID

**Returns**: `{"success": true}`

---

### `get_element_value`
Get the current value of an element.

**Parameters**:
- `element_id` (string): Automation ID

**Returns**: `{"value": "current_value"}`

**Supports**:
- Text inputs → text content
- Sliders → numeric value
- Checkboxes → "On"/"Off"/"Indeterminate"
- Other → element name

---

### `set_element_value`
Directly set the value of an element.

**Parameters**:
- `element_id` (string): Automation ID
- `value` (string): Value to set

**Returns**: `{"success": true}`

**Supports**:
- Text inputs (via Value pattern)
- Sliders (via RangeValue pattern, auto-converts to number)

---

### `type_text`
Focus an element and type text into it.

**Parameters**:
- `element_id` (string): Automation ID
- `text` (string): Text to type

**Returns**: `{"success": true}`

**Workflow**: focus → wait 50ms → set value

---

### `toggle_checkbox`
Toggle a checkbox state.

**Parameters**:
- `element_id` (string): Automation ID of checkbox

**Returns**: 
```json
{
  "success": true,
  "state": "On"  // or "Off", "Indeterminate"
}
```

---

### `set_checkbox`
Set checkbox to a specific state.

**Parameters**:
- `element_id` (string): Automation ID
- `checked` (boolean): true for checked, false for unchecked

**Returns**: `{"success": true, "checked": true}`

**Smart**: Only toggles if needed to reach desired state

---

### `select_combobox_option`
Select an option from a dropdown/combobox.

**Parameters**:
- `element_id` (string): Automation ID of combobox
- `option_text` (string): Text of option to select

**Returns**: `{"success": true, "selected": "option_text"}`

**Workflow**: Expands dropdown → finds option → selects it

---

## 🔧 Medium Priority Tools

### `get_checkbox_state`
Read the current state of a checkbox.

**Parameters**:
- `element_id` (string): Automation ID

**Returns**: `{"state": "On"}` // or "Off", "Indeterminate"

---

### `clear_text`
Clear all text from an input field.

**Parameters**:
- `element_id` (string): Automation ID

**Returns**: `{"success": true}`

---

### `get_selected_option`
Get currently selected option from combobox.

**Parameters**:
- `element_id` (string): Automation ID

**Returns**: `{"option": "selected_text"}`

---

### `select_tab`
Switch to a specific tab.

**Parameters**:
- `tab_name` (string): Display name of tab

**Returns**: `{"success": true, "tab": "tab_name"}`

---

### `get_active_tab`
Get the name of currently active tab.

**Parameters**: None

**Returns**: `{"tab": "active_tab_name"}`

---

### `get_window_list`
Get list of all application windows.

**Parameters**: None

**Returns**: `{"windows": ["Window 1", "Window 2"]}`

---

### `wait_for_element`
Wait for an element to appear.

**Parameters**:
- `element_id` (string): Automation ID
- `timeout_ms` (number): Max wait time in ms (default: 5000)

**Returns**: Full element info (same as `get_element`)

**Behavior**: Polls every 100ms until found or timeout

---

### `wait_for_element_stable`
Wait until an element's bounds stop changing for a stability window.

**Parameters**:
- `element_id` (string, required, non-empty): Element ID
- `stable_ms` (number, optional, default `300`): Required stable duration
- `timeout_ms` (number, optional, default `5000`): Max wait time
- `poll_interval_ms` (number, optional, default `100`): Poll interval

**Returns**: `{"success": true, "element": {...}}`

---

### `wait_for_value_change`
Wait until an element value changes.

**Parameters**:
- `element_id` (string, required, non-empty): Element ID
- `initial_value` (string, optional): Baseline value (if omitted, current value is used)
- `timeout_ms` (number, optional, default `5000`)
- `poll_interval_ms` (number, optional, default `100`)

**Returns**: `{"success": true, "value": "new_value"}`

---

### `find_nearest_element`
Find the nearest interactive element to screen coordinates.

**Parameters**:
- `x` (number, required): Screen X
- `y` (number, required): Screen Y
- `max_distance` (number, optional): Max distance in pixels

**Returns**:
```json
{
  "element": { "id": "...", "name": "...", "role": "...", "bounds": { "...": 0 } },
  "center": { "x": 0, "y": 0 },
  "distance": 12.3
}
```

---

### `right_click_element`
Open context menu by right-clicking an element center.

**Parameters**:
- `element_id` (string, required, non-empty)

**Returns**: `{"success": true, "x": 0, "y": 0}`

---

### `open_context_menu`
Alias of `right_click_element`.

**Parameters**:
- `element_id` (string, required, non-empty)

---

### `select_menu_item`
Find and activate a menu item by label.

**Parameters**:
- `label` (string, required, non-empty)
- `exact` (boolean, optional, default `true`)

**Returns**: `{"success": true, "item": {"id": "...", "name": "...", "role": "MenuItem"}, "used_menu_role": true}`

---

### `highlight_element`
Perform a quick visual highlight pass around an element by moving cursor around its bounds.

**Parameters**:
- `element_id` (string, required, non-empty)
- `delay_ms` (number, optional, default `40`)

**Returns**: `{"success": true, "element_id": "...", "bounds": {...}}`

---

### `flash_element`
Perform repeated visual flashing around an element by moving cursor around its bounds.

**Parameters**:
- `element_id` (string, required, non-empty)
- `flashes` (number, optional, default `3`)
- `delay_ms` (number, optional, default `60`)

**Returns**: `{"success": true, "element_id": "...", "flashes": 3}`

---

## 🖱️ Mouse & Keyboard Input Tools

### `click_at`
Click at specific screen coordinates.

**Parameters**:
- `x` (number): X coordinate
- `y` (number): Y coordinate

**Returns**: `{"success": true}`

---

### `double_click`
Double-click at coordinates.

**Parameters**:
- `x` (number): X coordinate
- `y` (number): Y coordinate

---

### `hover`
Move mouse to coordinates.

**Parameters**:
- `x` (number): X coordinate
- `y` (number): Y coordinate

---

### `drag`
Drag from one point to another.

**Parameters**:
- `x1`, `y1` (number): Start coordinates
- `x2`, `y2` (number): End coordinates

---

### `keyboard_input`
Send keyboard input.

**Parameters**:
- `key` (string): Key to press (e.g., "enter", "escape", "a", "tab")

**Supported Keys**: enter, escape, tab, backspace, delete, home, end, left, right, up, down, page_up, page_down, space, a-z, 0-9

---

### `scroll`
Scroll the mouse wheel.

**Parameters**:
- `delta_x` (number): Horizontal scroll
- `delta_y` (number): Vertical scroll (positive = up, negative = down)

---

## 📸 Screenshot Tools

### `take_screenshot`
Capture the entire screen.

**Parameters**: None

**Returns**:
```json
{
  "success": true,
  "image_base64": "data:image/png;base64,..."
}
```

---

### `screenshot_region`
Capture a specific region.

**Parameters**:
- `x`, `y` (number): Top-left corner
- `width`, `height` (number): Region size

**Returns**: Same as `take_screenshot`

---

## 🎛️ Convenience Tools

### `set_slider_value`
Find a slider and set its value.

**Parameters**:
- `value` (number): Percentage value (0-100)

**Returns**: 
```json
{
  "success": true,
  "value": 75,
  "message": "Slider set to 75"
}
```

**Behavior**: Finds first slider → clicks start → drags to target position

---

## 🔌 Connection Tools

### `ping`
Test server connectivity.

**Parameters**: None

**Returns**: `{"status": "pong"}`

---

### `check_connection`
Check IPC and UIA connection status.

**Parameters**: None

**Returns**:
```json
{
  "ipc_connected": true,
  "uia_connected": true,
  "uia_window_available": true,
  "status": "fully_connected"  // or "partial"
}
```

---

## 📖 Usage Examples

### Complete Form Workflow
```json
// 1. Focus text input
{"jsonrpc": "2.0", "id": 1, "method": "focus_element", 
 "params": {"element_id": "username"}}

// 2. Type username
{"jsonrpc": "2.0", "id": 2, "method": "type_text", 
 "params": {"element_id": "username", "text": "john_doe"}}

// 3. Check a checkbox
{"jsonrpc": "2.0", "id": 3, "method": "set_checkbox", 
 "params": {"element_id": "remember_me", "checked": true}}

// 4. Select from dropdown
{"jsonrpc": "2.0", "id": 4, "method": "select_combobox_option", 
 "params": {"element_id": "country", "option_text": "United States"}}

// 5. Click submit button
{"jsonrpc": "2.0", "id": 5, "method": "find_by_label", 
 "params": {"label": "Submit"}}
// Get bounds from response, then:
{"jsonrpc": "2.0", "id": 6, "method": "click_at", 
 "params": {"x": 150, "y": 300}}
```

### Tab Navigation
```json
// Check current tab
{"jsonrpc": "2.0", "id": 1, "method": "get_active_tab"}

// Switch tabs
{"jsonrpc": "2.0", "id": 2, "method": "select_tab", 
 "params": {"tab_name": "Advanced Settings"}}

// Wait for tab content to load
{"jsonrpc": "2.0", "id": 3, "method": "wait_for_element", 
 "params": {"element_id": "advanced_panel", "timeout_ms": 3000}}
```

### Dynamic Content Handling
```json
// Wait for element to appear after async operation
{"jsonrpc": "2.0", "id": 1, "method": "wait_for_element", 
 "params": {"element_id": "result_panel", "timeout_ms": 10000}}

// Get its value once it appears
{"jsonrpc": "2.0", "id": 2, "method": "get_element_value", 
 "params": {"element_id": "result_text"}}
```

---

## 🚨 Error Handling

All methods return errors in standard JSON-RPC format:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,  // Application error
    "message": "Failed to find element: Element with id 'missing_id' not found"
  }
}
```

**Common Error Codes**:
- `-32000`: Application error (element not found, pattern not supported, etc.)
- `-32001`: Service not available (IPC or UIA not connected)
- `-32602`: Invalid params (missing/empty/wrong-type required params)
- `-32601`: Method not found

---

## 🎯 Best Practices

1. **Always wait for elements** in dynamic UIs:
   ```json
   wait_for_element → then interact
   ```

2. **Use `focus_element` before keyboard input** for reliability

3. **Check connection status** at startup:
   ```json
   {"method": "check_connection"}
   ```

4. **Use pattern-specific tools** for better reliability:
   - `set_checkbox` instead of `click_at` for checkboxes
   - `select_combobox_option` instead of manual clicking
   - `type_text` instead of `keyboard_input` for text entry

5. **Handle async operations** with `wait_for_element`:
   ```json
   // After triggering async operation
   wait_for_element(loading_spinner, 500) → wait_for_element(result, 10000)
   ```

---

## 📊 Tool Categories Summary

- **Discovery**: 5 tools (get_ui_tree, find_by_label, find_by_role, get_element, get_window_list)
- **High Priority Interaction**: 7 tools (focus, get/set value, type, checkbox, combobox)
- **Medium Priority Interaction**: 7 tools (checkbox state, clear, tab navigation, wait)
- **Input**: 6 tools (click, double_click, hover, drag, keyboard, scroll)
- **Screenshot**: 2 tools (full screen, region)
- **Convenience**: 1 tool (set_slider_value)
- **Connection**: 2 tools (ping, check_connection)

**Total**: 30 tools available

---

## 🔄 Version Info

**Project**: egui-mcp Windows Port  
**Last Updated**: Implementation of all High + Medium priority tools  
**Build Status**: ✅ Compiled Successfully  
**Ready for**: Production testing


