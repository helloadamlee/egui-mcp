# egui-mcp for Windows

A Windows port of [egui-mcp](https://github.com/dijdzv/egui-mcp), enabling AI agents to interact with egui GUI applications through the Model Context Protocol (MCP).

**Key Difference**: The Linux version uses AT-SPI (Assistive Technology Service Provider Interface) over D-Bus, while this Windows version uses UI Automation (UIA) as its accessibility API.


## Current Project Status

- Canonical roadmap for remaining work: `ROADMAP_REMAINING.md`
- Canonical API/tool reference: `API_REFERENCE.md`

## Features

Note: This section lists the tool API surface and may include tools that depend on UIA readiness.
For current verified status, see `ROADMAP_REMAINING.md`.

### Working Tools

```
get_ui_tree
```

```
find_by_label
```

```
find_by_label_exact
```

```
find_by_role
```

```
get_element
```

```
click_element
```

```
get_bounds
```

```
focus_element
```

```
scroll_to_element
```

```
drag_element
```

```
get_text
```

```
get_caret_position
```

```
set_caret_position
```

```
get_text_selection
```

```
set_text_selection
```

```
get_value
```

```
set_value
```

```
get_selected_count
```

```
click_at
```

```
double_click
```

```
hover
```

```
drag
```

```
keyboard_input
```

```
scroll
```

```
take_screenshot
```

```
ping
```

```
check_connection
```

```
is_visible
```

```
is_enabled
```

```
is_focused
```

```
is_checked
```

```
screenshot_element
```

```
screenshot_region
```

```
wait_for_element
```

```
wait_for_state
```

```
compare_screenshots
```

```
diff_screenshots
```

```
highlight_element
```

```
clear_highlights
```

```
save_snapshot
```

```
load_snapshot
```

```
diff_snapshots
```

```
diff_current
```

```
get_frame_stats
```

```
start_perf_recording
```

```
get_perf_report
```

```
get_logs
```

```
clear_logs
```

* For ComboBox, checks the name property to determine if something is selected (returns 0 or 1).
** These tools require the element to have focus first. Use `focus_element` before calling. Returns -1 if no focus.
*** Requires the egui app to call `record_frame_auto()`. See Performance Metrics section.
**** Requires the egui app to be configured with `McpLogLayer`. See Log Access section.

### Not Working (Limitation)

The following tools are implemented but may not work due to various Windows-specific limitations:

```
set_text
```

```
keyboard_input
```

```
select_item
```

```
click_at
```

```
deselect_item
```

### Not Needed

The following tools are implemented but not useful for egui:

```
select_all
```

```
clear_selection
```

## Architecture

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
              │ COM/UIA API                 │ Named Pipe
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

## Requirements

* Windows 11 (recommended) or Windows 10
* Rust 1.85 or later
* Windows SDK (for UI Automation APIs)

## Installation

### Build from Source

```bash
git clone https://github.com/yourusername/egui-mcp-win.git
cd egui-mcp-win
cargo build --release
```

The compiled binary will be available at `target/release/egui-mcp-server-win.exe`

## Usage

### 1. Prepare Your egui Application

Enable AccessKit in your egui application:

```rust
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    
    eframe::run_native(
        "My App",
        options,
        Box::new(|cc| {
            // Enable AccessKit for UI Automation
            cc.egui_ctx.enable_accesskit();
            
            Ok(Box::new(MyApp::default()))
        }),
    )
}
```

### 2. Configure MCP Client

Add to your MCP client configuration (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "egui": {
      "command": "C:\\path\\to\\egui-mcp-server-win.exe",
      "args": []
    }
  }
}
```

### 3. Available Tools

See the [Working Tools](#working-tools) section above for a complete list of available MCP tools.

### Performance Metrics

To enable performance monitoring tools (`get_frame_stats`, `start_perf_recording`, `get_perf_report`), your egui app needs to call `record_frame_auto()`:

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Your UI code...
        
        // Record frame metrics
        self.runtime.block_on(self.mcp_client.record_frame_auto());
    }
}
```

### Log Access

To enable log-related tools (`get_logs`, `clear_logs`), configure your app with `McpLogLayer`:

```rust
use egui_mcp_client_win::McpLogLayer;
use tracing_subscriber::prelude::*;

fn main() {
    let (mcp_layer, log_buffer) = McpLogLayer::new(1000);
    
    tracing_subscriber::registry()
        .with(mcp_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Use log_buffer with your MCP client...
}
```

### Element Highlight

The `highlight_element` tool can draw overlay rectangles on your egui app. Draw highlights in your update loop:

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Your UI code...
        
        // Draw highlights
        let highlights = self.runtime.block_on(self.mcp_client.get_highlights());
        egui_mcp_client_win::draw_highlights(ctx, &highlights);
    }
}
```

## Windows-Specific Notes

### COM Initialization

UI Automation requires COM initialization. The server handles this automatically.

### Administrator Privileges

Some operations may require elevated privileges depending on the target application's integrity level.

### Security Software

Windows Defender or other antivirus software may flag automation tools. You may need to add exceptions.

### Testing Tools

Use **Inspect.exe** (included with Windows SDK) to verify the UI Automation tree:

```
"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\inspect.exe"
```

## Project Structure

```
egui-mcp-win/
|-- Cargo.toml
|-- README.md
|-- crates/
|   |-- egui-mcp-server-win/     # MCP server (UIA client)
|   |-- egui-mcp-client-win/     # Client library for egui apps
|   `-- egui-mcp-protocol/       # Shared protocol definitions
`-- .github/
    `-- workflows/
```

## Development

### Testing

```bash
# Run all tests
cargo test

# Run server tests
cargo test -p egui-mcp-server-win

# Run client tests
cargo test -p egui-mcp-client-win

# Run server MCP JSON-RPC integration tests
cargo test -p egui-mcp-server-win --test mcp_jsonrpc_integration
```

### Live UI Tests (Interactive Windows Session Required)

Live UI tests require a target egui application running with AccessKit enabled and a desktop session where UI Automation can access windows. This repository no longer includes an in-tree demo app.

```bash
# 1) Start your target egui app in one terminal
# 2) In another terminal, run the live Priority 2 MCP integration test
set EGUI_MCP_RUN_LIVE_TESTS=1
cargo test -p egui-mcp-server-win priority2_query_tools_succeed_against_live_demo_app -- --ignored --nocapture --test-threads=1

# 3) Run the live Priority 3 MCP integration test
cargo test -p egui-mcp-server-win priority3_tools_succeed_against_live_demo_app -- --ignored --nocapture --test-threads=1
```

CI configuration:
- `.github/workflows/windows-ci.yml` runs standard tests on `windows-latest`.

## Contributing

Contributions are welcome! Please see the original [egui-mcp](https://github.com/dijdzv/egui-mcp) repository for contribution guidelines.

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Related

* [egui-mcp (Linux version)](https://github.com/dijdzv/egui-mcp) - Original Linux implementation
* [egui](https://github.com/emilk/egui) - The immediate mode GUI library
* [AccessKit](https://github.com/AccessKit/accesskit) - Cross-platform accessibility toolkit
* [Model Context Protocol](https://modelcontextprotocol.io/) - Protocol specification

## Acknowledgments

This is a Windows port of the original [egui-mcp](https://github.com/dijdzv/egui-mcp) project. Special thanks to the original authors for their excellent work on the Linux version.


