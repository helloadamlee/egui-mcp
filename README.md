# egui-mcp for Windows

> **AI-powered UI automation for [egui](https://github.com/emilk/egui) desktop applications on Windows — via the [Model Context Protocol](https://modelcontextprotocol.io/).**

`egui-mcp` is a Rust-native MCP server that bridges AI agents and egui desktop applications running on Windows. It exposes a live UI tree through Windows UI Automation (UIA) as structured MCP tools, enabling AI agents to inspect, click, type, scroll, screenshot, and automate any egui app without modifying its business logic.

This is a **Windows port** of the original [egui-mcp](https://github.com/dijdzv/egui-mcp) (Linux/AT-SPI). The Linux version uses AT-SPI over D-Bus; this version replaces that with **Windows UI Automation (UIA) via COM**.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)](https://www.rust-lang.org/)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4?logo=windows)](#requirements)
[![MCP](https://img.shields.io/badge/protocol-MCP-blueviolet)](https://modelcontextprotocol.io/)

---

## Table of Contents

- [Why egui-mcp?](#why-egui-mcp)
- [How It Works](#how-it-works)
- [Requirements](#requirements)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [MCP Tool Reference](#mcp-tool-reference)
- [Optional Features](#optional-features)
- [Windows-Specific Notes](#windows-specific-notes)
- [Development](#development)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Related Projects](#related-projects)
- [License](#license)

---

## Why egui-mcp?

Testing, scripting, and automating egui GUIs traditionally required custom test harnesses or manual interaction. `egui-mcp` changes that by:

- **Exposing your egui UI as structured MCP tools** — any MCP-compatible AI client (Claude, Cursor, etc.) can discover and drive your app's UI in real time.
- **Requiring zero business-logic changes** — just call `enable_accesskit()` in your egui app and embed the lightweight client crate.
- **Supporting Windows natively** — using UI Automation (UIA) instead of AT-SPI, so it works on Windows 10 and Windows 11 without WSL or D-Bus.
- **Providing 30+ tools** — covering element discovery, interactions, screenshots, waits, performance metrics, and more.

---

## How It Works

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
              │ COM / UIA API               │ Named Pipe
              ▼                             ▼
┌─────────────────────────┐     ┌───────────────────────────────┐
│   UI Automation Tree    │     │   egui-mcp-client-win         │
│   (Windows built-in)    │     │   (embedded in egui app)      │
└─────────────────────────┘     └───────────────────────────────┘
              ▲                             ▲
              │ auto-published              │ embedded
              └─────────────────────────────┘
                        egui Application
                  (AccessKit → UIA bridge)
```

- **`egui-mcp-server-win`** — the MCP server binary that your AI client launches. It reads the UI via UIA and relays tool calls over stdin/stdout.
- **`egui-mcp-client-win`** — a lightweight library you embed in your egui app to enable screenshots, performance metrics, and log access over a named pipe.
- **AccessKit** — egui's built-in accessibility bridge. One call to `enable_accesskit()` is all that's needed to publish your UI tree to UIA.

---

## Requirements

| Requirement | Details |
|---|---|
| **OS** | Windows 10 or Windows 11 (recommended) |
| **Rust** | 1.85+ (edition 2024) |
| **Windows SDK** | Required for UI Automation APIs |
| **egui app** | Must call `cc.egui_ctx.enable_accesskit()` |

No additional services, daemons, or drivers are needed — Windows UI Automation is built into Windows.

---

## Installation

### Build from Source

```bash
git clone https://github.com/helloadamlee/egui-mcp.git
cd egui-mcp
cargo build --release
```

The server binary will be at:

```
target\release\egui-mcp-server-win.exe
```

---

## Quick Start

### Step 1 — Prepare your egui application

Add `egui-mcp-client-win` to your `Cargo.toml`:

```toml
[dependencies]
egui-mcp-client-win = { git = "https://github.com/helloadamlee/egui-mcp.git" }
tokio = { version = "1", features = ["full"] }
```

Enable AccessKit and embed the IPC server in your app entry point:

```rust
use egui_mcp_client_win::McpClient;

fn main() {
    let mcp_client = McpClient::new();

    // Start the named-pipe IPC server (enables screenshots, perf metrics, logs)
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client_clone = mcp_client.clone();
    runtime.spawn(async move {
        egui_mcp_client_win::IpcServer::run(client_clone).await.ok();
    });

    eframe::run_native("My App", Default::default(), Box::new(|cc| {
        // Publish the UI tree to Windows UI Automation
        cc.egui_ctx.enable_accesskit();
        Ok(Box::new(MyApp { mcp_client, runtime }))
    })).unwrap();
}
```

> **Note:** Calling `enable_accesskit()` is all that's required for UI tree access. The IPC server is only needed for screenshots, performance metrics, and log streaming.

### Step 2 — Configure your MCP client

Add the server to your MCP client configuration (e.g., Claude Desktop's `claude_desktop_config.json`):

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

### Step 3 — Start automating

Launch your egui app, then connect your MCP client. The AI agent can now discover and drive your UI using the tools below.

---

## MCP Tool Reference

`egui-mcp` exposes **30 tools** across 7 categories. For full parameter schemas and JSON-RPC examples, see [`API_REFERENCE.md`](./API_REFERENCE.md).

### UI Discovery

| Tool | Description |
|---|---|
| `get_ui_tree` | Get the complete UI element hierarchy as JSON |
| `find_by_label` | Find elements by display name (partial match) |
| `find_by_label_exact` | Find elements by exact display name |
| `find_by_role` | Find elements by control type (Button, Edit, Slider, etc.) |
| `get_element` | Get full details of an element by Automation ID |
| `get_window_list` | List all top-level application windows |

### Element Interaction

| Tool | Description |
|---|---|
| `focus_element` | Set keyboard focus to an element |
| `click_element` | Click an element by Automation ID |
| `type_text` | Focus an element and type text into it |
| `get_element_value` | Read the current value of an element |
| `set_element_value` | Set the value of a text input or slider |
| `toggle_checkbox` | Toggle a checkbox |
| `set_checkbox` | Set a checkbox to a specific state |
| `get_checkbox_state` | Read the current state of a checkbox |
| `clear_text` | Clear all text from an input field |
| `select_combobox_option` | Select a dropdown option by display text |
| `get_selected_option` | Get the currently selected dropdown option |
| `select_tab` | Switch to a named tab |
| `get_active_tab` | Get the name of the currently active tab |
| `right_click_element` | Right-click an element (opens context menu) |
| `open_context_menu` | Alias of `right_click_element` |
| `select_menu_item` | Find and activate a menu item by label |

### Wait & Polling

| Tool | Description |
|---|---|
| `wait_for_element` | Wait up to N ms for an element to appear |
| `wait_for_element_stable` | Wait until an element's bounds stop changing |
| `wait_for_value_change` | Wait until an element's value changes |
| `find_nearest_element` | Find the nearest interactive element to screen coordinates |

### Mouse & Keyboard Input

| Tool | Description |
|---|---|
| `click_at` | Click at screen coordinates |
| `double_click` | Double-click at screen coordinates |
| `hover` | Move the mouse to screen coordinates |
| `drag` | Drag from one point to another |
| `keyboard_input` | Send a keyboard key press |
| `scroll` | Scroll the mouse wheel |

### Screenshots & Visuals

| Tool | Description |
|---|---|
| `take_screenshot` | Capture the entire screen as PNG (base64) |
| `screenshot_region` | Capture a specific screen region |
| `highlight_element` | Visually highlight an element (cursor trace) |
| `flash_element` | Repeatedly flash an element for visual emphasis |

### Convenience

| Tool | Description |
|---|---|
| `set_slider_value` | Find the first slider and set it to a percentage (0–100) |

### Connection

| Tool | Description |
|---|---|
| `ping` | Verify the server is running |
| `check_connection` | Check UIA and IPC connection status |

---

## Optional Features

### Performance Metrics

To enable `get_frame_stats`, `start_perf_recording`, and `get_perf_report`, call `record_frame_auto()` at the end of each frame:

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... your UI ...

        // Record frame timing (1 line)
        self.runtime.block_on(self.mcp_client.record_frame_auto());
    }
}
```

### Log Access

To enable `get_logs` and `clear_logs`, configure `McpLogLayer` with `tracing`:

```rust
use egui_mcp_client_win::{McpClient, McpLogLayer};
use tracing_subscriber::prelude::*;

fn main() {
    let (mcp_layer, log_buffer) = McpLogLayer::new(1000); // keep last 1000 entries

    tracing_subscriber::registry()
        .with(mcp_layer)                        // capture for MCP access
        .with(tracing_subscriber::fmt::layer()) // also print to stdout
        .init();

    let mcp_client = McpClient::new().with_log_buffer_sync(log_buffer);
    // ... run egui app
}
```

### Element Highlighting

To enable `highlight_element` and `clear_highlights`, draw highlights at the end of your update loop:

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... your UI ...

        let highlights = self.runtime.block_on(self.mcp_client.get_highlights());
        egui_mcp_client_win::draw_highlights(ctx, &highlights);
    }
}
```

---

## Windows-Specific Notes

### COM Initialization

Windows UI Automation requires COM. The server initializes COM automatically — no action required.

### Administrator Privileges

Some automation operations may require elevated privileges if the target application runs at a higher integrity level (e.g., as Administrator). Run the server with matching privileges in that case.

### Security Software

Windows Defender or other AV tools may flag automation binaries. Add an exclusion for `egui-mcp-server-win.exe` if needed.

### Verifying the UI Tree

Use **Inspect.exe** (included with the Windows SDK) to verify what UI Automation exposes for your egui app:

```
"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\inspect.exe"
```

### Known Limitations

The following tools are implemented but have Windows-specific constraints:

| Tool | Issue | Workaround |
|---|---|---|
| `set_text` | AccessKit doesn't fully implement `IValueProvider` for all inputs | Use `type_text` |
| `select_item` | egui ComboBox child items may not be UIA-accessible | Use `select_combobox_option` |
| `keyboard_input` | Some key combos may not propagate to focused elements | Use `type_text` for text entry |

---

## Development

### Building

```bash
# Build all crates
cargo build --release

# Run all unit tests
cargo test

# Run server tests only
cargo test -p egui-mcp-server-win

# Run client tests only
cargo test -p egui-mcp-client-win

# Run JSON-RPC integration tests
cargo test -p egui-mcp-server-win --test mcp_jsonrpc_integration
```

### Live UI Tests

Live tests require a running egui app with AccessKit enabled and an interactive desktop session:

```bash
# Terminal 1: start your egui app

# Terminal 2: run live integration tests
set EGUI_MCP_RUN_LIVE_TESTS=1
cargo test -p egui-mcp-server-win priority2_query_tools_succeed_against_live_demo_app -- --ignored --nocapture --test-threads=1
cargo test -p egui-mcp-server-win priority3_tools_succeed_against_live_demo_app -- --ignored --nocapture --test-threads=1
```

CI runs standard (non-live) tests via `.github/workflows/windows-ci.yml` on `windows-latest`.

---

## Project Structure

```
egui-mcp/
├── crates/
│   ├── egui-mcp-server-win/   # MCP server binary (UIA client)
│   ├── egui-mcp-client-win/   # Library for egui apps (IPC server)
│   └── egui-mcp-protocol/     # Shared protocol definitions
├── .github/
│   └── workflows/
│       └── windows-ci.yml     # CI pipeline
├── API_REFERENCE.md           # Full tool schemas and JSON-RPC examples
├── Cargo.toml
└── README.md
```

---

## Contributing

Contributions are welcome! Please follow [Conventional Commits](https://www.conventionalcommits.org/) for commit messages:

| Prefix | Effect |
|---|---|
| `feat:` | New feature (bumps minor version) |
| `fix:` | Bug fix (bumps patch version) |
| `feat!:` | Breaking change (bumps major version) |
| `docs:` | Documentation only |
| `refactor:` | Code restructuring |
| `test:` | Test additions or changes |
| `chore:` | Build, CI, dependencies |

---

## Related Projects

| Project | Description |
|---|---|
| [egui-mcp (Linux)](https://github.com/dijdzv/egui-mcp) | Original Linux implementation using AT-SPI |
| [egui](https://github.com/emilk/egui) | Immediate mode GUI library for Rust |
| [AccessKit](https://github.com/AccessKit/accesskit) | Cross-platform accessibility abstraction for Rust GUIs |
| [Model Context Protocol](https://modelcontextprotocol.io/) | Open protocol for AI–tool integration |
| [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) | Native + web app framework built on egui |

---

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

---

## Acknowledgments

This project is a Windows port of [egui-mcp](https://github.com/dijdzv/egui-mcp) by [@dijdzv](https://github.com/dijdzv). Many thanks for the original work on the Linux version that made this port possible.
