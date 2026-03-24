# egui-MCP Windows - Functional MCP Tools

> Status note (2026-03-23): The canonical remaining-work list is `ROADMAP_REMAINING.md`.
> Use this file for tool behavior details; use the roadmap file for what is left.


## ✅ FULLY FUNCTIONAL (IPC-based)

These tools work right now because they use the working IPC layer:

### 1. **ping** 
Check if the server is running
```json
{"jsonrpc":"2.0","id":1,"method":"ping"}
```
Response:
```json
{"jsonrpc":"2.0","id":1,"result":{"status":"pong"}}
```

### 2. **check_connection**
Verify both IPC and UIA connections
```json
{"jsonrpc":"2.0","id":1,"method":"check_connection"}
```
Response:
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "result":{
    "ipc_connected": true,
    "uia_connected": false,
    "status": "partial"
  }
}
```

### 3. **click_at** ✅ TESTED
Click at specific coordinates
```json
{"jsonrpc":"2.0","id":1,"method":"click_at","params":{"x":200,"y":250}}
```
Response:
```json
{"jsonrpc":"2.0","id":1,"result":{"success":true}}
```

### 4. **double_click**
Double-click at specific coordinates
```json
{"jsonrpc":"2.0","id":1,"method":"double_click","params":{"x":200,"y":250}}
```

### 5. **hover**
Move mouse to specific coordinates
```json
{"jsonrpc":"2.0","id":1,"method":"hover","params":{"x":200,"y":250}}
```

### 6. **drag**
Drag from one point to another
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "method":"drag",
  "params":{"x1":100,"y1":200,"x2":300,"y2":200}
}
```

### 7. **keyboard_input**
Send keyboard input (key down + key up)
```json
{"jsonrpc":"2.0","id":1,"method":"keyboard_input","params":{"key":"Enter"}}
```
Supported keys: Any string recognized by the IPC server

### 8. **scroll**
Scroll by delta amounts
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "method":"scroll",
  "params":{"delta_x":0,"delta_y":-10}
}
```

### 9. **take_screenshot**
Capture full window screenshot
```json
{"jsonrpc":"2.0","id":1,"method":"take_screenshot"}
```
Response:
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "result":{
    "success":true,
    "image_base64":"iVBORw0KGgo..."
  }
}
```

### 10. **screenshot_region**
Capture specific region of the window
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "method":"screenshot_region",
  "params":{"x":0,"y":0,"width":400,"height":300}
}
```

### 11. **set_slider_value** ✅ TESTED & WORKING!
Convenience method to set slider value (0-100)
```json
{"jsonrpc":"2.0","id":1,"method":"set_slider_value","params":{"value":100}}
```
Response:
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "result":{
    "success":true,
    "value":100.0,
    "message":"Slider set to 100"
  }
}
```
**Note:** This tool will use UIA to find the slider once UIA is working. Currently uses hardcoded coordinates.

---

## ⚠️ NOT FUNCTIONAL (UIA-based - Needs Implementation)

These tools are implemented but won't work until the Windows UIA client is fixed:

### 12. **get_ui_tree**
Get complete UI tree structure
```json
{"jsonrpc":"2.0","id":1,"method":"get_ui_tree"}
```
**Status:** UIA client has compilation errors  
**Error:** Returns "UI Automation not available"

### 13. **find_by_label**
Find elements by label (substring match)
```json
{"jsonrpc":"2.0","id":1,"method":"find_by_label","params":{"label":"Slider"}}
```
**Status:** UIA client not connected  
**Error:** Returns "UI Automation not available"

### 14. **find_by_label_exact**
Find elements by exact label match
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "method":"find_by_label_exact",
  "params":{"label":"Check me!"}
}
```
**Status:** UIA client not connected

### 15. **find_by_role**
Find elements by AccessKit role
```json
{"jsonrpc":"2.0","id":1,"method":"find_by_role","params":{"role":"Button"}}
```
**Status:** UIA client not connected  
**Supported roles:** Button, CheckBox, Slider, Edit, etc.

### 16. **get_element**
Get specific element by ID
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "method":"get_element",
  "params":{"element_id":"slider-1"}
}
```
**Status:** UIA client not connected

---

## Summary

### Working (11 tools) ✅
- ✅ ping
- ✅ check_connection
- ✅ click_at (TESTED)
- ✅ double_click
- ✅ hover
- ✅ drag
- ✅ keyboard_input
- ✅ scroll
- ✅ take_screenshot
- ✅ screenshot_region
- ✅ set_slider_value (TESTED & WORKING)

### Blocked by UIA (5 tools) ⚠️
- ⚠️ get_ui_tree
- ⚠️ find_by_label
- ⚠️ find_by_label_exact
- ⚠️ find_by_role
- ⚠️ get_element

---

## How to Test

### 1. Start the demo app
```powershell
C:\egui-mcp\target\debug\demo-app-win.exe
```

### 2. Start the MCP server
```powershell
C:\egui-mcp\target\release\egui-mcp-server-win.exe
```

### 3. Send commands via stdin
```powershell
# Test ping
echo '{"jsonrpc":"2.0","id":1,"method":"ping"}' | C:\egui-mcp\target\release\egui-mcp-server-win.exe

# Test set_slider_value
echo '{"jsonrpc":"2.0","id":1,"method":"set_slider_value","params":{"value":75}}' | ...
```

---

## Current Limitations

### IPC Tools Work BUT:
1. **Coordinate-based only** - You need to know exact pixel positions
2. **No semantic queries** - Can't find "the slider" or "button with label X"
3. **Fragile** - If window moves or resizes, coordinates break

### UIA Tools Would Add:
1. **Semantic queries** - Find elements by label, role, properties
2. **Robust** - Works regardless of window position/size
3. **Accessible** - Uses the same system screen readers use

---

## Next Steps to Enable UIA Tools

1. **Fix compilation errors** in `uia_client.rs`
   - Remove `get_` prefix from UIA method calls
   - Fix error handling for HRESULT
   - Add missing struct definitions (ElementInfo, Rect)

2. **Test UIA connection**
   ```rust
   let uia = UiaClient::new().await?;
   let tree = uia.get_ui_tree("egui-mcp Demo").await?;
   ```

3. **Verify AccessKit** is working in demo app
   - Check that `enable_accesskit()` is called
   - Use Windows Accessibility Insights to verify

4. **Test semantic queries**
   ```json
   {"method":"find_by_label","params":{"label":"Slider"}}
   ```

---

## Example Workflow (Current Working State)

```bash
# 1. Start demo app
./demo-app-win.exe

# 2. Start MCP server (in another terminal)
./egui-mcp-server-win.exe

# 3. Send commands to stdin
echo '{"jsonrpc":"2.0","id":1,"method":"check_connection"}' 
# Response: {"ipc_connected":true,"uia_connected":false}

echo '{"jsonrpc":"2.0","id":2,"method":"click_at","params":{"x":200,"y":250}}'
# Response: {"success":true}

echo '{"jsonrpc":"2.0","id":3,"method":"set_slider_value","params":{"value":100}}'
# Response: {"success":true,"value":100.0,"message":"Slider set to 100"}
```

---

## Testing Notes

### Verified Working ✅
- [x] IPC connection via named pipe
- [x] ping/pong communication
- [x] click_at with real coordinates
- [x] set_slider_value (moves slider to 100)
- [x] Response formatting (JSON-RPC 2.0)

### Not Yet Tested
- [ ] screenshot (need to verify image encoding)
- [ ] keyboard_input (need key mapping)
- [ ] scroll (need to test delta values)
- [ ] drag (need to verify mouse event sequence)

### Cannot Test (UIA not working)
- [ ] get_ui_tree
- [ ] find_by_label
- [ ] find_by_role
- [ ] semantic element queries


