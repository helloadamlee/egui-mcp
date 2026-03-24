# egui-mcp for Windows 11 - Project Guide

## Overview

This document provides a comprehensive guide for creating a Windows 11 port of [egui-mcp](https://github.com/dijdzv/egui-mcp), which enables AI agents to interact with egui GUI applications through the Model Context Protocol (MCP).

**Key Difference**: The Linux version uses AT-SPI (Assistive Technology Service Provider Interface) over D-Bus, while Windows uses UI Automation (UIA) as its accessibility API.

---

## Architecture Comparison

### Linux (Original egui-mcp)
```
┌───────────────────────────────────────────────────────────────┐
│                    MCP Client (AI Agent)                      │
└────────────────────────────┬──────────────────────────────────┘
                             │ MCP Protocol (stdio)
                             ▼
┌───────────────────────────────────────────────────────────────┐
│                      egui-mcp-server                          │
│  ┌─────────────────────┐      ┌─────────────────────┐         │
│  │   AT-SPI Client     │      │    IPC Client       │         │
│  │ (UI tree & actions) │      │   (screenshots)     │         │
│  └──────────┬──────────┘      └───────────┬─────────┘         │
└─────────────┼─────────────────────────────┼───────────────────┘
              │ D-Bus                       │ Unix Socket
              ▼                             ▼
┌─────────────────────────┐     ┌───────────────────────────────┐
│      AT-SPI Bus         │     │     egui-mcp-client           │
└─────────────────────────┘     └───────────────────────────────┘
              ▲                             ▲
              │ auto-publish                │ embedded
┌───────────────────────────────────────────────────────────────┐
│              egui Application (AccessKit → AT-SPI)            │
└───────────────────────────────────────────────────────────────┘
```

### Windows 11 (Target Architecture)
```
┌───────────────────────────────────────────────────────────────┐
│                    MCP Client (AI Agent)                      │
└────────────────────────────┬──────────────────────────────────┘
                             │ MCP Protocol (stdio)
                             ▼
┌───────────────────────────────────────────────────────────────┐
│                   egui-mcp-server-win                         │
│  ┌─────────────────────┐      ┌─────────────────────┐         │
│  │   UIA Client        │      │    IPC Client       │         │
│  │ (UI tree & actions) │      │   (screenshots)     │         │
│  └──────────┬──────────┘      └───────────┬─────────┘         │
└─────────────┼─────────────────────────────┼───────────────────┘
              │ COM/UIA API                 │ Named Pipe / TCP
              ▼                             ▼
┌─────────────────────────┐     ┌───────────────────────────────┐
│   UI Automation Tree    │     │   egui-mcp-client-win         │
└─────────────────────────┘     └───────────────────────────────┘
              ▲                             ▲
              │ auto-publish                │ embedded
┌───────────────────────────────────────────────────────────────┐
│              egui Application (AccessKit → UIA)               │
└───────────────────────────────────────────────────────────────┘
```

---

## Project Structure

```
egui-mcp-win/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── docs/
│   └── windows-uia-investigation.md
├── crates/
│   ├── egui-mcp-server-win/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs              # MCP server entry point
│   │       ├── lib.rs               # Core library
│   │       ├── uia_client.rs        # UI Automation client wrapper
│   │       ├── tools/               # MCP tool implementations
│   │       │   ├── mod.rs
│   │       │   ├── tree.rs          # get_ui_tree, find_by_*
│   │       │   ├── interaction.rs   # click_element, focus, etc.
│   │       │   ├── text.rs          # get_text, set_text, etc.
│   │       │   ├── value.rs         # get_value, set_value
│   │       │   └── input.rs         # click_at, keyboard_input, etc.
│   │       └── ipc.rs               # IPC client for screenshots/input
│   ├── egui-mcp-client-win/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs               # Client library for egui apps
│   │       ├── ipc_server.rs        # Named pipe/TCP server
│   │       ├── screenshot.rs        # Screenshot capture via Windows APIs
│   │       ├── input.rs             # SendInput wrapper
│   │       ├── highlight.rs         # Overlay drawing
│   │       └── perf.rs              # Performance metrics
│   └── egui-mcp-protocol/           # Shared (can reuse from Linux)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── messages.rs          # IPC message types
│           └── types.rs             # Shared types
└── examples/
    └── demo-app-win/
        ├── Cargo.toml
        └── src/
            └── main.rs              # Demo egui application
```

---

## Core Dependencies

### For `egui-mcp-server-win`

```toml
[package]
name = "egui-mcp-server-win"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"

[dependencies]
# Windows UI Automation bindings
uiautomation = "0.5"
windows = { version = "0.58", features = [
    "Win32_UI_Accessibility",
    "Win32_Foundation",
    "Win32_System_Com",
    "Win32_UI_WindowsAndMessaging",
] }

# Alternative: Use windows-rs directly for more control
# windows-sys = { version = "0.59", features = [
#     "Win32_UI_Accessibility",
#     "Win32_System_Com",
# ] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# MCP SDK
mcp-sdk = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Protocol definitions (local workspace)
egui-mcp-protocol = { path = "../egui-mcp-protocol" }
```

### For `egui-mcp-client-win`

```toml
[package]
name = "egui-mcp-client-win"
version = "0.1.0"
edition = "2021"

[dependencies]
# Windows APIs for screenshots and input
windows = { version = "0.58", features = [
    "Win32_Graphics_Gdi",
    "Win32_Graphics_Imaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Foundation",
] }

# Async runtime
tokio = { version = "1", features = ["sync", "net", "time"] }

# Image processing
image = "0.25"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# egui for drawing highlights
egui = "0.29"

# Protocol definitions (local workspace)
egui-mcp-protocol = { path = "../egui-mcp-protocol" }
```

### For `demo-app-win`

```toml
[package]
name = "demo-app-win"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.29"
egui = "0.29"
egui-mcp-client-win = { path = "../../crates/egui-mcp-client-win" }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

#### 1.1 Setup Project Structure
- [ ] Create workspace with all crates
- [ ] Set up Cargo.toml files
- [ ] Configure Windows-specific features
- [ ] Set up basic CI/CD for Windows

#### 1.2 UI Automation Client Wrapper
- [ ] Create `uia_client.rs` wrapping `uiautomation` crate
- [ ] Implement connection to UI Automation tree
- [ ] Test with simple Windows applications (Notepad, Calculator)
- [ ] Document UIA tree traversal patterns

**Example Code:**
```rust
// crates/egui-mcp-server-win/src/uia_client.rs

use uiautomation::{UIAutomation, UIElement};
use windows::Win32::UI::Accessibility::*;
use anyhow::Result;

pub struct UiaClient {
    automation: UIAutomation,
}

impl UiaClient {
    pub fn new() -> Result<Self> {
        let automation = UIAutomation::new()?;
        Ok(Self { automation })
    }

    pub fn get_root(&self) -> Result<UIElement> {
        self.automation.get_root_element()
    }

    pub fn find_by_name(&self, name: &str) -> Result<Vec<UIElement>> {
        let root = self.get_root()?;
        let condition = self.automation
            .create_property_condition(UIA_NamePropertyId, name)?;
        let elements = root.find_all(TreeScope_Descendants, &condition)?;
        Ok(elements)
    }
}
```

#### 1.3 IPC Layer
- [ ] Choose IPC method (Named Pipes recommended for Windows)
- [ ] Implement async server in `egui-mcp-client-win`
- [ ] Implement client in `egui-mcp-server-win`
- [ ] Test bidirectional communication

**Named Pipe Server Example:**
```rust
// crates/egui-mcp-client-win/src/ipc_server.rs

use tokio::net::windows::named_pipe::{ServerOptions, NamedPipeServer};
use anyhow::Result;

const PIPE_NAME: &str = r"\\.\pipe\egui_mcp_client";

pub async fn run_ipc_server() -> Result<()> {
    loop {
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(PIPE_NAME)?;
        
        server.connect().await?;
        
        tokio::spawn(async move {
            handle_connection(server).await
        });
    }
}

async fn handle_connection(mut pipe: NamedPipeServer) -> Result<()> {
    // Handle IPC messages
    Ok(())
}
```

### Phase 2: Core MCP Tools (Week 3-4)

#### 2.1 Tree Inspection Tools
- [ ] `get_ui_tree` - Full UI tree via UIA
- [ ] `find_by_label` - Search by name property
- [ ] `find_by_label_exact` - Exact name match
- [ ] `find_by_role` - Search by control type
- [ ] `get_element` - Get element by automation ID

**Implementation Pattern:**
```rust
// crates/egui-mcp-server-win/src/tools/tree.rs

use uiautomation::UIElement;
use serde_json::{json, Value};

pub async fn get_ui_tree(uia_client: &UiaClient) -> Result<Value> {
    let root = uia_client.get_root()?;
    build_tree_recursive(&root, 0)
}

fn build_tree_recursive(element: &UIElement, depth: i32) -> Result<Value> {
    let name = element.get_name()?;
    let control_type = element.get_control_type()?;
    let automation_id = element.get_automation_id()?;
    let bounding_rect = element.get_bounding_rectangle()?;
    
    let mut children = Vec::new();
    if depth < 20 { // Prevent infinite recursion
        for child in element.get_children()? {
            children.push(build_tree_recursive(&child, depth + 1)?);
        }
    }
    
    Ok(json!({
        "name": name,
        "role": control_type_to_string(control_type),
        "automation_id": automation_id,
        "bounds": {
            "x": bounding_rect.left,
            "y": bounding_rect.top,
            "width": bounding_rect.right - bounding_rect.left,
            "height": bounding_rect.bottom - bounding_rect.top,
        },
        "children": children,
    }))
}
```

#### 2.2 Element Interaction Tools
- [ ] `click_element` - Invoke pattern
- [ ] `focus_element` - Set focus
- [ ] `get_bounds` - Bounding rectangle
- [ ] `is_visible` - Check visibility state
- [ ] `is_enabled` - Check enabled state
- [ ] `is_focused` - Check focus state

**Click Implementation:**
```rust
// crates/egui-mcp-server-win/src/tools/interaction.rs

use uiautomation::types::UIPattern;
use windows::Win32::UI::Accessibility::UIA_InvokePatternId;

pub async fn click_element(
    uia_client: &UiaClient,
    automation_id: &str
) -> Result<()> {
    let element = uia_client.find_by_automation_id(automation_id)?;
    
    // Try Invoke pattern first (for buttons)
    if let Ok(invoke) = element.get_pattern::<IUIAutomationInvokePattern>(
        UIA_InvokePatternId
    ) {
        invoke.Invoke()?;
        return Ok(());
    }
    
    // Fallback to click at center
    let bounds = element.get_bounding_rectangle()?;
    let center_x = (bounds.left + bounds.right) / 2;
    let center_y = (bounds.top + bounds.bottom) / 2;
    
    // Use SendInput for mouse click
    send_mouse_click(center_x, center_y)?;
    Ok(())
}
```

### Phase 3: Input & Screenshot (Week 5)

#### 3.1 Coordinate-based Input (IPC)
- [ ] `click_at` - SendInput wrapper
- [ ] `double_click` - Double click via SendInput
- [ ] `hover` - Move mouse cursor
- [ ] `drag` - Drag operation
- [ ] `keyboard_input` - Keyboard events
- [ ] `scroll` - Mouse wheel events

**SendInput Wrapper:**
```rust
// crates/egui-mcp-client-win/src/input.rs

use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::Foundation::*;

pub fn send_mouse_click(x: i32, y: i32) -> Result<()> {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    
    // Convert to absolute coordinates (0-65535 range)
    let abs_x = (x * 65535 / screen_width) as i32;
    let abs_y = (y * 65535 / screen_height) as i32;
    
    let mut inputs = [
        // Move mouse
        INPUT {
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
        },
        // Press left button
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // Release left button
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
    
    Ok(())
}
```

#### 3.2 Screenshot Capture
- [ ] `take_screenshot` - Capture via GDI/D3D
- [ ] `screenshot_element` - Element-specific capture
- [ ] `screenshot_region` - Rectangular region capture
- [ ] Return as base64 or save to file

**Screenshot Implementation:**
```rust
// crates/egui-mcp-client-win/src/screenshot.rs

use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::*;
use image::{ImageBuffer, Rgba};

pub fn capture_window(hwnd: HWND) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    unsafe {
        let hdc_window = GetDC(hwnd);
        let hdc_mem = CreateCompatibleDC(hdc_window);
        
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect)?;
        
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        
        let hbitmap = CreateCompatibleBitmap(hdc_window, width, height)?;
        SelectObject(hdc_mem, hbitmap);
        
        BitBlt(
            hdc_mem,
            0, 0,
            width, height,
            hdc_window,
            0, 0,
            SRCCOPY,
        )?;
        
        // Convert bitmap to image buffer
        let img = bitmap_to_image(hbitmap, width, height)?;
        
        // Cleanup
        DeleteObject(hbitmap);
        DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_window);
        
        Ok(img)
    }
}
```

### Phase 4: Advanced Features (Week 6-7)

#### 4.1 Text Manipulation
- [ ] `get_text` - Text pattern
- [ ] `set_text` - Value pattern (limited by egui)
- [ ] `get_caret_position`
- [ ] `set_caret_position`
- [ ] `get_text_selection`
- [ ] `set_text_selection`

#### 4.2 Value Patterns
- [ ] `get_value` - For sliders, progress bars
- [ ] `set_value` - For adjustable controls

#### 4.3 Waiting & Polling
- [ ] `wait_for_element` - Poll until element appears
- [ ] `wait_for_state` - Poll until state changes

#### 4.4 Visual Comparison
- [ ] `compare_screenshots` - Image similarity score
- [ ] `diff_screenshots` - Visual diff generation
- [ ] `highlight_element` - Draw overlay rectangle
- [ ] `clear_highlights`

#### 4.5 State Management
- [ ] `save_snapshot` - Save UI tree state
- [ ] `load_snapshot` - Load saved state
- [ ] `diff_snapshots` - Compare two states
- [ ] `diff_current` - Compare with current state

### Phase 5: Demo Application (Week 8)

#### 5.1 Create Demo egui App
- [ ] Setup basic egui application
- [ ] Enable AccessKit (`enable_accesskit()`)
- [ ] Integrate `egui-mcp-client-win`
- [ ] Add various UI elements for testing
- [ ] Add performance recording
- [ ] Add log capture

**Demo App Structure:**
```rust
// examples/demo-app-win/src/main.rs

use eframe::egui;
use egui_mcp_client_win::{McpClient, McpLogLayer, IpcServer};
use tracing_subscriber::prelude::*;

fn main() -> Result<(), eframe::Error> {
    // Set up logging with MCP layer
    let (mcp_layer, log_buffer) = McpLogLayer::new(1000);
    
    tracing_subscriber::registry()
        .with(mcp_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Create MCP client
    let mcp_client = McpClient::new().with_log_buffer_sync(log_buffer);
    
    // Start IPC server in background
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client_clone = mcp_client.clone();
    runtime.spawn(async move {
        IpcServer::run(client_clone).await.ok();
    });
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "egui MCP Demo (Windows)",
        options,
        Box::new(|cc| {
            // Enable AccessKit for UI Automation
            cc.egui_ctx.enable_accesskit();
            
            Ok(Box::new(DemoApp::new(mcp_client, runtime)))
        }),
    )
}

struct DemoApp {
    mcp_client: McpClient,
    runtime: tokio::runtime::Runtime,
    
    // UI state
    text_input: String,
    slider_value: f32,
    checkbox: bool,
    radio_selection: usize,
}

impl DemoApp {
    fn new(mcp_client: McpClient, runtime: tokio::runtime::Runtime) -> Self {
        Self {
            mcp_client,
            runtime,
            text_input: String::new(),
            slider_value: 0.5,
            checkbox: false,
            radio_selection: 0,
        }
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui MCP Demo (Windows)");
            
            ui.separator();
            
            // Text input
            ui.horizontal(|ui| {
                ui.label("Text Input:");
                ui.text_edit_singleline(&mut self.text_input);
            });
            
            // Slider
            ui.add(egui::Slider::new(&mut self.slider_value, 0.0..=1.0)
                .text("Slider"));
            
            // Checkbox
            ui.checkbox(&mut self.checkbox, "Checkbox");
            
            // Radio buttons
            ui.horizontal(|ui| {
                ui.label("Radio:");
                ui.radio_value(&mut self.radio_selection, 0, "Option A");
                ui.radio_value(&mut self.radio_selection, 1, "Option B");
                ui.radio_value(&mut self.radio_selection, 2, "Option C");
            });
            
            // Buttons
            if ui.button("Click Me").clicked() {
                tracing::info!("Button clicked!");
            }
            
            if ui.button("Show Message").clicked() {
                tracing::info!("Message button clicked");
            }
        });
        
        // Record frame for performance metrics
        self.runtime.block_on(self.mcp_client.record_frame_auto());
        
        // Draw highlights
        let highlights = self.runtime.block_on(self.mcp_client.get_highlights());
        egui_mcp_client_win::draw_highlights(ctx, &highlights);
    }
}
```

### Phase 6: Testing & Documentation (Week 9-10)

#### 6.1 Testing
- [ ] Unit tests for UIA client
- [ ] Integration tests with demo app
- [ ] Test all MCP tools
- [ ] Test with Windows Accessibility Insights
- [ ] Performance benchmarks

#### 6.2 Documentation
- [ ] README with Windows-specific setup
- [ ] API documentation
- [ ] Tool usage examples
- [ ] Troubleshooting guide
- [ ] Comparison with Linux version

---

## Key API Mappings

### AT-SPI (Linux) → UI Automation (Windows)

| AT-SPI Concept | Windows UIA Equivalent | Notes |
|----------------|------------------------|-------|
| `get_tree()` | `FindAll(TreeScope_Descendants)` | Full tree traversal |
| `get_interfaces()` | `GetCurrentPattern()` | Element capabilities |
| `do_action()` | `IUIAutomationInvokePattern::Invoke()` | Button clicks |
| `get_text()` | `IUIAutomationTextPattern::GetText()` | Text content |
| `get_value()` | `IUIAutomationValuePattern::GetValue()` | Slider/input values |
| `set_value()` | `IUIAutomationValuePattern::SetValue()` | Modify values |
| `get_name()` | `IUIAutomationElement::CurrentName` | Element label |
| `get_role()` | `IUIAutomationElement::CurrentControlType` | Element type |
| `get_state()` | `IUIAutomationElement::GetCurrentPattern()` | Various states |
| `get_position()` | `IUIAutomationElement::CurrentBoundingRectangle` | Screen position |

### Control Type Mappings

| egui Widget | AccessKit Role | UIA Control Type |
|-------------|----------------|------------------|
| Button | Button | UIA_ButtonControlTypeId |
| TextEdit | TextInput | UIA_EditControlTypeId |
| Checkbox | CheckBox | UIA_CheckBoxControlTypeId |
| RadioButton | RadioButton | UIA_RadioButtonControlTypeId |
| Slider | Slider | UIA_SliderControlTypeId |
| ComboBox | ComboBox | UIA_ComboBoxControlTypeId |
| Label | StaticText | UIA_TextControlTypeId |
| Window | Window | UIA_WindowControlTypeId |

---

## Windows-Specific Considerations

### 1. COM Initialization
UI Automation requires COM to be initialized:

```rust
use windows::Win32::System::Com::*;

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;
    }
    
    // ... your code
    
    unsafe {
        CoUninitialize();
    }
    
    Ok(())
}
```

### 2. Administrator Privileges
Some operations may require elevated privileges. Consider:
- Running as admin for testing
- Documenting privilege requirements
- Gracefully handling permission errors

### 3. Window Handles (HWND)
You'll need to find egui application windows:

```rust
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn find_window_by_title(title: &str) -> Option<HWND> {
    unsafe {
        FindWindowW(None, title)
    }
}
```

### 4. DPI Awareness
Handle different DPI settings:

```rust
use windows::Win32::UI::HiDpi::*;

fn init_dpi_awareness() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}
```

### 5. Security & Sandboxing
Windows may block automation APIs based on:
- UIPI (User Interface Privilege Isolation)
- Process integrity levels
- Antivirus/security software

---

## Testing Tools

### Inspect.exe
Built into Windows SDK, use to verify UIA tree:
```
"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\inspect.exe"
```

### Accessibility Insights for Windows
Microsoft's testing tool:
- Download: https://accessibilityinsights.io/
- Verify AccessKit exposure
- Test automation patterns

### UISpy (Legacy)
Older but still useful:
- Included in Windows SDK
- Good for debugging UIA issues

---

## Common Pitfalls & Solutions

### 1. AccessKit Not Publishing to UIA
**Problem**: `enable_accesskit()` called but no UIA tree visible

**Solution**:
- Ensure eframe is using Windows backend (not WASM)
- Check AccessKit version compatibility
- Verify COM is initialized
- Test with Inspect.exe

### 2. IPC Connection Failures
**Problem**: Named pipe connection refused

**Solution**:
- Check pipe name format (`\\.\pipe\name`)
- Ensure server starts before client connects
- Handle ERROR_PIPE_BUSY with retry logic
- Consider TCP fallback for easier debugging

### 3. SendInput Not Working
**Problem**: Input simulation has no effect

**Solution**:
- Check UIPI (User Interface Privilege Isolation)
- Run both apps at same privilege level
- Verify window has focus
- Use absolute coordinates (0-65535 range)

### 4. Screenshot Captures Wrong Window
**Problem**: Screenshot shows desktop instead of app

**Solution**:
- Ensure HWND is correct
- Use `GetClientRect` not `GetWindowRect`
- Handle DPI scaling
- Verify window is not minimized

---

## Performance Considerations

### 1. UIA Tree Traversal
- Cache frequently accessed elements
- Limit tree depth (prevent infinite recursion)
- Use focused scope when possible
- Batch multiple queries

### 2. IPC Latency
- Use binary protocol (not JSON) for large data
- Compress screenshots before transmission
- Consider shared memory for large transfers
- Pool connections

### 3. Screenshot Capture
- Use D3D11 capture for better performance than GDI
- Downsample screenshots if full resolution not needed
- Cache unchanged regions
- Consider hardware acceleration

---

## Next Steps

1. **Week 1**: Setup project structure and UIA client wrapper
2. **Week 2**: Implement IPC layer with named pipes
3. **Week 3**: Core tree inspection tools
4. **Week 4**: Element interaction tools
5. **Week 5**: Input simulation and screenshots
6. **Week 6**: Advanced features (text, value patterns)
7. **Week 7**: Waiting, comparison, and state tools
8. **Week 8**: Demo application
9. **Week 9**: Comprehensive testing
10. **Week 10**: Documentation and polish

---

## Resources

### Documentation
- [Windows UI Automation Overview](https://learn.microsoft.com/en-us/windows/win32/winauto/windows-automation-api-portal)
- [UI Automation Providers](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-providersoverview)
- [AccessKit Documentation](https://docs.rs/accesskit/)
- [egui Accessibility](https://docs.rs/egui/latest/egui/struct.Context.html#method.enable_accesskit)

### Rust Crates
- [uiautomation](https://crates.io/crates/uiautomation) - UI Automation bindings
- [windows](https://crates.io/crates/windows) - Windows API bindings
- [mcp-sdk](https://crates.io/crates/mcp-sdk) - Model Context Protocol SDK
- [tokio](https://crates.io/crates/tokio) - Async runtime

### Tools
- Windows SDK (includes Inspect.exe, UISpy)
- Accessibility Insights for Windows
- Visual Studio (debugging COM/UIA)

### Community
- [egui Discord](https://discord.gg/JFcEma9bJq)
- [AccessKit GitHub Discussions](https://github.com/AccessKit/accesskit/discussions)
- Rust Windows Dev Community

---

## License

This project guide is provided as reference material. The original egui-mcp project is licensed under MIT OR Apache-2.0.

---

## Contact & Contributions

For questions or contributions to this Windows port effort:
- Open issues on the repository
- Join discussions in relevant Discord servers
- Submit PRs following conventional commits

**Happy Building! 🚀**
