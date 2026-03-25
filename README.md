# egui-mcp for Windows

> AI-powered UI automation for [egui](https://github.com/emilk/egui) apps on Windows via the [Model Context Protocol](https://modelcontextprotocol.io/).

A Windows port of [egui-mcp](https://github.com/dijdzv/egui-mcp) (Linux/AT-SPI). The original uses AT-SPI over D-Bus; this version replaces that with **Windows UI Automation (UIA) via COM** — no WSL, no D-Bus required.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)](https://www.rust-lang.org/)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4?logo=windows)](#requirements)

---

## How It Works

```
MCP Client (AI Agent)
       │ stdio
       ▼
egui-mcp-server-win
  ├── UIA Client (COM)      → UI tree & element actions
  └── IPC Client ( Named Pipe) → screenshots, perf, logs
       ▲
egui Application  →  enable_accesskit()  →  AccessKit  →  UIA
```

---

## Linux vs Windows: What Changed

| Feature | Linux (original) | Windows (this port) |
|---|---|---|
| **Accessibility API** | AT-SPI over D-Bus | Windows UI Automation (COM) |
| **IPC transport** | Unix socket | Named pipe |
| **Platform** | Linux only | Windows 10 / 11 |
| **Installation** | `cargo install egui-mcp-server` (crates.io) | Build from source |
| `get_ui_tree` / `find_by_*` / `get_element` | ✅ | ✅ |
| `click_element`, `focus_element` | ✅ | ✅ |
| `click_at`, `hover`, `drag`, `scroll`, `keyboard_input` | ✅ | ✅ |
| `take_screenshot`, `screenshot_region` | ✅ | ✅ |
| `highlight_element`, `clear_highlights` | ✅ AT-SPI + IPC | ✅ cursor-trace |
| `wait_for_element` | ✅ | ✅ |
| `ping`, `check_connection` | ✅ | ✅ |
| `get_bounds`, `scroll_to_element`, `drag_element` | ✅ AT-SPI | ❌ removed |
| `get_text`, `get/set_caret_position`, `get/set_text_selection` | ✅ AT-SPI Text | ❌ removed |
| `get_value`, `set_value` | ✅ AT-SPI Value | ❌ replaced by `set_element_value` |
| `is_visible`, `is_enabled`, `is_focused`, `is_checked` | ✅ AT-SPI State | ❌ removed |
| `screenshot_element` | ✅ | ✅ restored |
| `wait_for_state` | ✅ | ❌ removed |
| `compare_screenshots`, `diff_screenshots` | ✅ | ✅ restored |
| `save/load/diff_snapshots` | ✅ | ✅ restored (`diff_current` pending) |
| `get_frame_stats`, `start_perf_recording`, `get_perf_report` | ✅ | ❌ removed |
| `get_logs`, `clear_logs` | ✅ | ❌ removed |
| **`get_window_list`** | ❌ | ✅ **new** |
| **`type_text`** | ❌ | ✅ **new** — focus + set value in one call |
| **`get_element_value` / `set_element_value`** | ❌ | ✅ **new** — UIA Value pattern |
| **`toggle_checkbox` / `set_checkbox` / `get_checkbox_state`** | ❌ | ✅ **new** — UIA Toggle pattern |
| **`clear_text`** | ❌ | ✅ **new** |
| **`select_combobox_option` / `get_selected_option`** | ❌ | ✅ **new** — UIA Expand/Select pattern |
| **`select_tab` / `get_active_tab`** | ❌ | ✅ **new** |
| **`right_click_element` / `open_context_menu` / `select_menu_item`** | ❌ | ✅ **new** |
| **`wait_for_element_stable` / `wait_for_value_change`** | ❌ | ✅ **new** |
| **`find_nearest_element`** | ❌ | ✅ **new** |
| **`flash_element`** | ❌ | ✅ **new** |
| **`set_slider_value`** | ❌ | ✅ **new** — percentage-based convenience tool |

---

## Requirements

- Windows 10 or 11
- Rust 1.85+
- Windows SDK (for UIA APIs)
- Your egui app must call `cc.egui_ctx.enable_accesskit()`

---

## Installation

```bash
git clone https://github.com/helloadamlee/egui-mcp.git
cd egui-mcp
cargo build --release
# → target\release\egui-mcp-server-win.exe
```

---

## Quick Start

**1. Add the client crate to your egui app:**

```toml
[dependencies]
egui-mcp-client-win = { git = "https://github.com/helloadamlee/egui-mcp.git" }
tokio = { version = "1", features = ["full"] }
```

```rust
use egui_mcp_client_win::McpClient;

fn main() {
    let mcp_client = McpClient::new();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client_clone = mcp_client.clone();
    runtime.spawn(async move {
        egui_mcp_client_win::IpcServer::run(client_clone).await.ok();
    });
    eframe::run_native("My App", Default::default(), Box::new(|cc| {
        cc.egui_ctx.enable_accesskit(); // publishes UI tree to UIA
        Ok(Box::new(MyApp { mcp_client, runtime }))
    })).unwrap();
}
```

**2. Configure your MCP client** (e.g. Claude Desktop):

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

**3.** Launch your egui app, connect your MCP client, and start automating.

For full parameter schemas and JSON-RPC examples, see [`API_REFERENCE.md`](./API_REFERENCE.md).

---

## Windows Notes

- **COM** is initialized automatically by the server.
- **Elevated privileges** may be needed if the target app runs as Administrator.
- **Antivirus** — add an exclusion for `egui-mcp-server-win.exe` if flagged.
- **Inspect.exe** (Windows SDK) is useful for verifying the UIA tree exposed by your app.

### Known Limitations

| Tool | Issue | Workaround |
|---|---|---|
| `set_text` | `IValueProvider` not fully implemented by AccessKit | Use `type_text` |
| `select_item` | egui ComboBox children may not expose to UIA | Use `select_combobox_option` |

---

## Development

```bash
cargo test                          # all tests
cargo test -p egui-mcp-server-win   # server only
cargo test -p egui-mcp-server-win --test mcp_jsonrpc_integration
cargo test -p egui-mcp-server-win kittest_ -- --nocapture

# Screenshot compare/diff logic is shared by:
# - runtime MCP handlers (compare_screenshots, diff_screenshots, snapshots)
# - deterministic egui_kittest tests (fast validation, consistent behavior)

# Live UI tests (requires a running egui app)
set EGUI_MCP_RUN_LIVE_TESTS=1
cargo test -p egui-mcp-server-win priority2_... -- --ignored --nocapture --test-threads=1
```

CI: `.github/workflows/windows-ci.yml` on `windows-latest`.

---

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

Upstream project: [dijdzv/egui-mcp](https://github.com/dijdzv/egui-mcp). Thanks to the original authors.

