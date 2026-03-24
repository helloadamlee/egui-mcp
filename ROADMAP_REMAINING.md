# eGui MCP Windows Roadmap Remaining

Last updated: 2026-03-24

This file is the single source of truth for current Windows port status and remaining work.
Historical snapshot/status files were removed during repository cleanup on 2026-03-24.

## Progress Update (2026-03-24)

Completed in this pass:
- Priority 1.1 started and partially completed:
  - Hardened COM/UIA startup in `crates/egui-mcp-server-win/src/uia_client.rs`:
    - Handles `RPC_E_CHANGED_MODE` explicitly.
    - Only calls `CoUninitialize` when this client initialized COM.
  - Improved window discovery reliability:
    - Case-insensitive/partial title matching.
    - Retry window lookup for short startup races.
    - Better diagnostics including visible top-level windows.
  - Updated server connection reporting in `crates/egui-mcp-server-win/src/lib.rs`:
    - `check_connection` now reports `uia_window_available`.
- Priority 1.3 started:
  - Added UIA unit/smoke tests in `uia_client.rs` (role mapping, title matching).
  - Added ignored live UIA smoke tests for startup + pattern support checks.
- Priority 4.1 improved:
  - Fixed `examples/demo-app-win/tests/demo_app_tests.rs` for `egui_kittest 0.31` API.
  - `cargo test -p demo-app-win` now passes.
- Priority 2 started and partially verified:
  - Added stricter parameter validation for `find_by_label`, `find_by_label_exact`, `find_by_role`, and `get_element` (JSON-RPC `-32602` for missing/invalid params).
  - Added role validation and descriptive unsupported-role errors for `find_by_role`.
  - Fixed element identity fallback by using UIA runtime IDs when automation IDs are empty.
- Added ignored live smoke test covering all Priority 2 query tools (`uia_live_priority2_query_tools_smoke`).
- Executed live smoke tests locally against a launched `demo-app-win` and confirmed pass:
  - `uia_live_priority2_query_tools_smoke`
  - `uia_live_window_lookup_and_pattern_support_smoke`
- Priority 4.2 started:
  - Added server-process MCP JSON-RPC integration tests in `crates/egui-mcp-server-win/tests/mcp_jsonrpc_integration.rs`.
  - Non-live integration tests now verify:
    - `ping` round-trip
    - `check_connection` response shape
    - invalid-param behavior (`-32602`) for Priority 2 query tools
    - `get_ui_tree` method registration (not `-32601`)
  - Added ignored live MCP integration test for full Priority 2 success path against `demo-app-win`.
  - Validated live-test command path locally:
    - Guarded skip path (`EGUI_MCP_RUN_LIVE_TESTS=1` without interactive-session hint).
    - Forced execution path (`EGUI_MCP_RUN_LIVE_TESTS=1`, `EGUI_MCP_FORCE_LIVE_TESTS=1`) with `demo-app-win` running.
- Priority 3 started:
  - Added new tools/handlers for:
    - `find_nearest_element`
    - `right_click_element`
    - `open_context_menu`
    - `select_menu_item`
    - `highlight_element`
    - `flash_element`
    - `wait_for_element_stable`
    - `wait_for_value_change`
  - Added supporting unit tests for nearest-element and flash-path helper logic.
  - Added ignored live MCP integration test for Priority 3 tool path (`priority3_tools_succeed_against_live_demo_app`).
  - Added `Actions` menu items in `demo-app-win` so `select_menu_item` has a real target.
  - Executed live Priority 3 smoke test locally with forced live mode and confirmed pass (including `select_menu_item`).
  - Added clipboard round-trip integration test (`set_clipboard` + `get_clipboard`) with retries, lock serialization, and clipboard restoration.

Still remaining for Milestone A/B:
- Validate the new live-test workflow on an available self-hosted interactive Windows runner (`.github/workflows/windows-ci.yml`).

## Priority 1: Unblock Semantic UI Automation

1. Fix Windows UIA client stability and connection path in `crates/egui-mcp-server-win/src/uia_client.rs`.
2. Verify UIA connectivity end-to-end with the demo app (`enable_accesskit`) and MCP server.
3. Add/refresh tests for UIA startup, element lookup, and pattern support detection.

Why this is first:
- Coordinate-based tools work now, but semantic automation depends on UIA.

## Priority 2: Make UIA-Dependent MCP Tools Fully Functional

The following tools are listed as blocked by UIA and should be verified functional after the UIA fix:
1. `get_ui_tree`
2. `find_by_label`
3. `find_by_label_exact`
4. `find_by_role`
5. `get_element`

Definition of done:
- Tool responds successfully against `demo-app-win`.
- Error paths are descriptive for missing elements/unsupported patterns.
- Tool behavior is documented in API docs.

## Priority 3: Complete Remaining Interaction Tools

Planned but not fully implemented/verified across docs:
1. Menu and context interaction tools.
2. Clipboard tools (`get_clipboard`, `set_clipboard`).
3. Spatial/precision tools (for example `find_nearest_element`, `click_center`).
4. Visual debug tools (highlight/flash element).
5. Advanced wait conditions (`wait_for_element_stable`, `wait_for_value_change`).

## Priority 4: Testing and Quality Gate

1. Add/finish `egui_kittest` coverage for demo app behavior.
2. Add integration tests for MCP JSON-RPC tool execution.
3. Validate negative paths (timeouts, read-only controls, unsupported UIA patterns).
4. Ensure CI runs tests for server/client/demo app on Windows.

## Priority 5: Docs and Cleanup

1. Reconcile status files so all docs report the same current state.
2. Update `README.md` and `API_REFERENCE.md` with authoritative tool status.
3. Keep one status owner file (suggested: this file) and link all other docs to it.

## Status Ownership

1. Use this file for current status and remaining work.
2. Use `API_REFERENCE.md` for API shape/parameters/examples.
3. Use `README.md` for setup and top-level project guidance.

## Suggested Milestone Sequence

1. Milestone A: UIA core fixed and connected.
2. Milestone B: 5 blocked UIA query tools verified.
3. Milestone C: Advanced interaction tool set completed.
4. Milestone D: Test coverage and CI hardening.
5. Milestone E: Documentation consolidation.
