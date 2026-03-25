# Windows Screenshot Tool Restoration Plan (Core 4)

## Summary
Restore the screenshot feature set in a Windows-native way, covering:
- `screenshot_element`
- `compare_screenshots`
- `diff_screenshots`
- snapshot persistence tools (`save_snapshot`, `load_snapshot`, `diff_snapshots`) with server-managed storage

This pass also standardizes all screenshot outputs to **PNG bytes encoded in `image_base64`** (strict mode, no legacy raw RGB path).

## Implementation Changes
- **Screenshot encoding contract**
  - Replace current raw RGB-in-base64 pipeline with true PNG encoding before returning `image_base64`.
  - Apply consistently to `take_screenshot`, `screenshot_region`, and all new screenshot-returning tools.
  - Keep response field name `image_base64` for compatibility at the schema level.

- **`screenshot_element`**
  - Resolve element bounds via UIA (`element_id` required).
  - Clamp capture rect to screen bounds; return clear error if resulting rect is empty/off-screen.
  - Capture via existing region screenshot path (single capture backend).

- **`compare_screenshots`**
  - Inputs: two screenshot payloads (base64 PNG) and optional threshold.
  - Decode both PNGs, normalize to same dimensions policy:
    - default: strict equal size required (error if mismatch).
  - Compute similarity score (pixel-level normalized difference) and boolean `match` against threshold.
  - Return: `similarity`, `match`, `width`, `height`.

- **`diff_screenshots`**
  - Inputs: two screenshot payloads (base64 PNG), optional highlight color and sensitivity.
  - Decode, enforce same-size requirement, compute per-pixel difference mask.
  - Produce a PNG diff image (base64) with highlighted changed regions.
  - Return: `diff_image_base64`, `changed_pixels`, `change_ratio`.

- **Snapshot tools (server-managed folder)**
  - Add server snapshot store under a fixed directory (e.g., app data or repo-local runtime dir).
  - `save_snapshot`: capture/store PNG with generated snapshot ID and optional name/label metadata.
  - `load_snapshot`: retrieve snapshot image+metadata by ID/name.
  - `diff_snapshots`: load two stored snapshots and run the same diff engine as `diff_screenshots`.
  - Use atomic writes and metadata index file to prevent corruption.

- **MCP server/API wiring**
  - Register and route new methods in the server dispatcher.
  - Validate required params (`-32602`), runtime failures (`-32000`), unavailable service (`-32001`).
  - Update docs for method contracts and output examples to PNG-based semantics.

## Test Plan
- **Unit tests**
  - PNG encode/decode round-trip validity.
  - Compare algorithm correctness on identical vs intentionally modified images.
  - Diff output generation (non-empty diff for changed input, empty/near-empty for identical input).
  - Snapshot ID generation and metadata serialization/deserialization.

- **Integration tests (non-live)**
  - Invalid params for each new method (`-32602`).
  - Compare/diff with mismatched dimensions returns deterministic error.
  - Snapshot save/load/diff lifecycle works end-to-end.
  - Existing `take_screenshot`/`screenshot_region` now return decodable PNG bytes.

- **Live tests (ignored/opt-in)**
  - `screenshot_element` on a known visible element succeeds.
  - Compare/diff detect a real UI state change across two captures.
  - Snapshot tools work against a running target app.

## Assumptions and Defaults
- Scope is **Core 4** only (capture+compare+diff+snapshot tools), excluding extra screenshot-adjacent features for this pass.
- Snapshot storage is **server-managed folder** with generated IDs (no caller-provided path requirement).
- Output image contract is **strict PNG base64 in `image_base64`** for all screenshot outputs.
- Compatibility mode is **strict PNG only** (no legacy raw RGB fallback).
- Dimension policy for compare/diff is **strict same-size required** in v1.
