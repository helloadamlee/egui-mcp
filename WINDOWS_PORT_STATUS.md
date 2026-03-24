# egui-MCP Windows Port - Architecture & Kittest Integration

> Status note (2026-03-23): The canonical remaining-work list is `ROADMAP_REMAINING.md`.
> This document is a historical architecture/status snapshot.

## Project Status: ✅ WORKING!

### What We Accomplished Today

1. **Named Pipe IPC** - ✅ Working communication between MCP server and demo app
2. **Slider Control** - ✅ Successfully set slider to 100 via MCP protocol  
3. **Basic Architecture** - ✅ Server connects via IPC and responds to MCP commands

### Test Command
```json
{"jsonrpc":"2.0","id":1,"method":"set_slider_value","params":{"value":100}}
```

Response:
```json
{"id":1,"jsonrpc":"2.0","result":{"message":"Slider set to 100","success":true,"value":100.0}}
```

---

## Architecture Overview

### Current Working Setup

```
┌─────────────────────────────────────────────────────────┐
│              MCP Client (AI Agent)                      │
│           (stdin/stdout JSON-RPC)                       │
└────────────────────┬────────────────────────────────────┘
                     │
                     │ MCP Protocol
                     ▼
┌─────────────────────────────────────────────────────────┐
│        egui-mcp-server-win.exe                          │
│                                                         │
│  ┌──────────────────┐      ┌──────────────────┐        │
│  │  UIA Client      │      │   IPC Client     │        │
│  │  (UI tree -      │      │ (screenshots &   │        │
│  │   TODO)          │      │  input events)   │        │
│  └────────┬─────────┘      └────────┬─────────┘        │
└───────────┼──────────────────────────┼──────────────────┘
            │                          │
            │ Windows UIA              │ Named Pipe
            │ (not connected yet)      │ \\.\pipe\egui_mcp_client
            │                          │
            ▼                          ▼
┌─────────────────────────────────────────────────────────┐
│           demo-app-win.exe                              │
│                                                         │
│  ┌──────────────────┐      ┌──────────────────┐        │
│  │  AccessKit       │      │ IPC Server       │        │
│  │  (UIA Provider)  │◄────►│ (Named Pipe)     │        │
│  │enable_accesskit()│      │                  │        │
│  └──────────────────┘      └──────────────────┘        │
└─────────────────────────────────────────────────────────┘
```

---

## egui_kittest vs egui-MCP

### egui_kittest (Official egui Testing Library)

**Purpose:** In-process testing of egui applications  
**Location:** https://github.com/emilk/egui/tree/main/crates/egui_kittest  
**Maintained by:** egui core team  

**Use Cases:**
- ✅ Unit testing egui UI components
- ✅ Integration testing
- ✅ Regression testing with snapshots
- ✅ Automated GUI testing in CI/CD

**How It Works:**
```rust
use egui_kittest::{Harness, kittest::Queryable};

let mut harness = Harness::new_ui(app);
let checkbox = harness.get_by_label("Check me!");
checkbox.click();
harness.run();
```

**Key Features:**
- Semantic queries (`get_by_label`, `get_by_role`)
- Event simulation (`click()`, `type_text()`)
- AccessKit integration (cross-platform)
- Snapshot testing support

### egui-MCP (Your Project)

**Purpose:** External AI agent control of egui applications  
**Maintained by:** You!  

**Use Cases:**
- ✅ AI agent automation (Claude, GPT, etc.)
- ✅ Out-of-process control
- ✅ Remote GUI automation
- ✅ Cross-application workflows

**How It Works:**
```bash
# MCP server running
egui-mcp-server-win.exe

# AI sends JSON-RPC commands
{"method":"set_slider_value","params":{"value":100}}

# Server controls app via named pipe
IPC → demo-app-win.exe
```

---

## Why Both Are Needed

| Feature | egui_kittest | egui-MCP |
|---------|--------------|----------|
| **Process** | In-process | Out-of-process |
| **Purpose** | Developer testing | AI agent control |
| **Query Method** | Direct AccessKit | Windows UIA / IPC |
| **Use Case** | CI/CD, unit tests | AI automation, demos |
| **Installation** | `cargo add egui_kittest` | Custom server binary |

### They're Complementary!

1. **egui_kittest** - For **you** to test your app works correctly
2. **egui-MCP** - For **AI agents** to control your app

---

## Implementation Status

### ✅ Working
- [x] Named pipe IPC communication
- [x] IPC Server in demo app
- [x] IPC Client in MCP server  
- [x] Basic MCP protocol handling
- [x] Slider control via coordinates

### 🚧 In Progress
- [ ] Windows UIA client implementation
- [ ] UI tree query tools
- [ ] Element-based interaction (vs coordinate-based)

### 📋 TODO
- [ ] Fix UIA client compilation errors
- [ ] Implement `find_by_label`, `find_by_role`
- [ ] Verify and stabilize screenshot tools across scenarios
- [ ] Complete remaining MCP tool set (see `ROADMAP_REMAINING.md`)
- [ ] Add egui_kittest tests for demo app

---

## Next Steps

### Option 1: Fix Windows UIA Client (Recommended)
This gives you semantic UI queries from the MCP server.

**Files to fix:**
- `crates/egui-mcp-server-win/src/uia_client.rs`
- `crates/egui-mcp-server-win/src/lib.rs`

**Benefits:**
- Query UI by label/role instead of coordinates
- More robust automation
- Better AI agent control

### Option 2: Add egui_kittest Tests
Show how to test the demo app properly.

**Files to create:**
- `examples/demo-app-win/tests/demo_app_tests.rs` (already created!)

**Benefits:**
- Verify UI behavior
- Regression testing
- CI/CD integration

### Option 3: Harden Existing IPC Tools
Stabilize and validate existing IPC tools across resolutions and window states.

**Focus areas:**
- take_screenshot reliability and encoding checks
- keyboard_input key mapping coverage
- drag and hover behavior across DPI/scaling

---

## Files Modified Today

### Created/Updated
1. `crates/egui-mcp-server-win/src/ipc.rs` - ✅ Full named pipe client
2. `crates/egui-mcp-server-win/src/lib.rs` - ✅ MCP server with IPC
3. `crates/egui-mcp-server-win/src/uia_client.rs` - ⚠️ Partial (has errors)
4. `examples/demo-app-win/tests/demo_app_tests.rs` - ✅ Example kittest tests
5. `examples/demo-app-win/Cargo.toml` - ✅ Added egui_kittest dependency

### Working Binaries
- `target/release/egui-mcp-server-win.exe` - MCP server
- `target/debug/demo-app-win.exe` - Demo application

---

## References

### Official Resources
- **egui_kittest**: https://github.com/emilk/egui/tree/main/crates/egui_kittest
- **kittest**: https://github.com/rerun-io/kittest
- **egui**: https://github.com/emilk/egui
- **AccessKit**: https://github.com/AccessKit/accesskit
- **Original egui-mcp (Linux)**: https://github.com/dijdzv/egui-mcp

### Your Project
- **Location**: `C:\egui-mcp\`
- **Server**: `crates/egui-mcp-server-win`
- **Client**: `crates/egui-mcp-client-win`
- **Demo**: `examples/demo-app-win`

---

## Key Takeaways

1. **Your Windows port is working!** 🎉
   - Named pipe IPC ✅
   - MCP protocol ✅
   - Basic automation ✅

2. **egui_kittest != egui-MCP**
   - Different purposes
   - Different architectures
   - Both valuable!

3. **Next priority: Windows UIA**
   - AccessKit exposes Windows UIA
   - Your MCP server should query it
   - Enables semantic queries

4. **Testing strategy:**
   - Use egui_kittest for developer tests
   - Use egui-MCP for AI agent control

---

## Questions?

- UIA implementation help?
- More MCP tools?
- egui_kittest examples?

Just ask! 😊





