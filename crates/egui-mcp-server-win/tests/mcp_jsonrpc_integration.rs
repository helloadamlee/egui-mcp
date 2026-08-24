use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStderr, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use serde_json::{json, Value};

const SERVER_START_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const LIVE_TESTS_ENV: &str = "EGUI_MCP_RUN_LIVE_TESTS";
const FORCE_LIVE_TESTS_ENV: &str = "EGUI_MCP_FORCE_LIVE_TESTS";
const CLIPBOARD_RETRY_ATTEMPTS: usize = 5;
const CLIPBOARD_RETRY_DELAY: Duration = Duration::from_millis(150);

struct ServerHarness {
    child: Child,
    stdin: ChildStdin,
    responses: Receiver<Value>,
    _reader_thread: thread::JoinHandle<()>,
    _stderr_thread: thread::JoinHandle<()>,
}

impl ServerHarness {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_egui-mcp-server-win"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn egui-mcp-server-win");

        let stdin = child.stdin.take().expect("missing child stdin");
        let stdout = child.stdout.take().expect("missing child stdout");
        let stderr = child.stderr.take().expect("missing child stderr");

        let (tx, rx) = mpsc::channel::<Value>();
        let reader_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if let Ok(value) = serde_json::from_str::<Value>(line.trim()) {
                            let _ = tx.send(value);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let stderr_thread = spawn_stderr_drain(stderr);

        let mut harness = Self {
            child,
            stdin,
            responses: rx,
            _reader_thread: reader_thread,
            _stderr_thread: stderr_thread,
        };

        harness.await_server_ready();
        harness
    }

    fn await_server_ready(&mut self) {
        let start = Instant::now();
        let mut id = 9_000;

        while start.elapsed() < SERVER_START_TIMEOUT {
            let ping = self.send_request(id, "ping", None);
            match ping {
                Ok(response) => {
                    if response
                        .get("result")
                        .and_then(|value| value.get("status"))
                        .and_then(Value::as_str)
                        == Some("pong")
                    {
                        return;
                    }
                }
                Err(_) => thread::sleep(Duration::from_millis(200)),
            }
            id += 1;
        }

        panic!(
            "server did not become ready within {:?}",
            SERVER_START_TIMEOUT
        );
    }

    fn send_request(
        &mut self,
        id: i64,
        method: &str,
        params: Option<Value>,
    ) -> std::result::Result<Value, String> {
        let mut payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method
        });
        if let Some(params) = params {
            payload["params"] = params;
        }

        let line = serde_json::to_string(&payload)
            .map_err(|err| format!("failed to serialize request: {}", err))?;
        writeln!(self.stdin, "{line}")
            .and_then(|_| self.stdin.flush())
            .map_err(|err| format!("failed to write request: {}", err))?;

        let deadline = Instant::now() + REQUEST_TIMEOUT;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self
                .responses
                .recv_timeout(remaining.min(Duration::from_millis(500)))
            {
                Ok(response) => {
                    if response.get("id").and_then(Value::as_i64) == Some(id) {
                        return Ok(response);
                    }
                }
                Err(_) => {
                    if self.child.try_wait().ok().flatten().is_some() {
                        return Err("server process exited before response".to_string());
                    }
                }
            }
        }

        Err(format!(
            "timed out waiting for response for method '{}' (id={})",
            method, id
        ))
    }

    /// Sends a JSON-RPC notification, which carries no `id` and must not be
    /// answered.
    fn send_notification(&mut self, method: &str, params: Option<Value>) {
        let mut payload = json!({
            "jsonrpc": "2.0",
            "method": method
        });
        if let Some(params) = params {
            payload["params"] = params;
        }

        let line = serde_json::to_string(&payload).expect("failed to serialize notification");
        writeln!(self.stdin, "{line}")
            .and_then(|_| self.stdin.flush())
            .expect("failed to write notification");
    }

    /// True when no target app is reachable, meaning a blind sweep across every
    /// tool cannot drive real mouse or keyboard input.
    fn is_isolated(&mut self) -> bool {
        let response = self
            .send_request(8_100, "check_connection", None)
            .expect("check_connection failed");
        let result = response.get("result").expect("check_connection missing result");

        let ipc = result
            .get("ipc_connected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let window = result
            .get("uia_window_available")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        !ipc && !window
    }
}

impl Drop for ServerHarness {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn spawn_stderr_drain(stderr: ChildStderr) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    })
}

fn assert_error_code(response: &Value, expected: i64) {
    let code = response
        .get("error")
        .and_then(|err| err.get("code"))
        .and_then(Value::as_i64)
        .expect("missing error.code");
    assert_eq!(
        code, expected,
        "unexpected error code in response: {response:#}"
    );
}

fn assert_error_message_contains(response: &Value, expected_substring: &str) {
    let message = response
        .get("error")
        .and_then(|err| err.get("message"))
        .and_then(Value::as_str)
        .expect("missing error.message");
    assert!(
        message.contains(expected_substring),
        "expected error message to contain '{}', got '{}'",
        expected_substring,
        message
    );
}

fn response_error_message(response: &Value) -> Option<String> {
    let error = response.get("error")?;
    if let Some(message) = error.get("message").and_then(Value::as_str) {
        return Some(message.to_string());
    }
    Some(error.to_string())
}

fn png_base64(width: u32, height: u32, color: [u8; 4]) -> String {
    let image = RgbaImage::from_fn(width, height, |_x, _y| Rgba(color));
    let mut cursor = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .expect("encode png");
    general_purpose::STANDARD.encode(cursor.into_inner())
}

fn clipboard_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn set_clipboard_with_retry(
    server: &mut ServerHarness,
    base_id: i64,
    text: &str,
) -> std::result::Result<(), String> {
    let mut last_error = String::new();

    for attempt in 0..CLIPBOARD_RETRY_ATTEMPTS {
        let request_id = base_id + attempt as i64;
        match server.send_request(request_id, "set_clipboard", Some(json!({ "text": text }))) {
            Ok(response) => {
                if response.get("error").is_none() {
                    return Ok(());
                }
                last_error = response_error_message(&response)
                    .unwrap_or_else(|| "unknown set_clipboard error".to_string());
            }
            Err(err) => last_error = err,
        }
        thread::sleep(CLIPBOARD_RETRY_DELAY);
    }

    Err(format!(
        "set_clipboard failed after {} attempts: {}",
        CLIPBOARD_RETRY_ATTEMPTS, last_error
    ))
}

fn get_clipboard_with_retry(
    server: &mut ServerHarness,
    base_id: i64,
) -> std::result::Result<String, String> {
    let mut last_error = String::new();

    for attempt in 0..CLIPBOARD_RETRY_ATTEMPTS {
        let request_id = base_id + attempt as i64;
        match server.send_request(request_id, "get_clipboard", None) {
            Ok(response) => {
                if let Some(error) = response_error_message(&response) {
                    last_error = error;
                } else if let Some(text) = response
                    .get("result")
                    .and_then(|result| result.get("text"))
                    .and_then(Value::as_str)
                {
                    return Ok(text.to_string());
                } else {
                    last_error = "get_clipboard response missing result.text".to_string();
                }
            }
            Err(err) => last_error = err,
        }
        thread::sleep(CLIPBOARD_RETRY_DELAY);
    }

    Err(format!(
        "get_clipboard failed after {} attempts: {}",
        CLIPBOARD_RETRY_ATTEMPTS, last_error
    ))
}

fn live_tests_enabled() -> bool {
    std::env::var(LIVE_TESTS_ENV)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn force_live_tests_enabled() -> bool {
    std::env::var(FORCE_LIVE_TESTS_ENV)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn has_interactive_session_hint() -> bool {
    std::env::var("SESSIONNAME")
        .map(|value| {
            let value = value.to_ascii_lowercase();
            value == "console" || value.starts_with("rdp-")
        })
        .unwrap_or(false)
}

#[test]
fn ping_round_trip_returns_pong() {
    let mut server = ServerHarness::spawn();
    let response = server.send_request(1, "ping", None).expect("ping failed");

    assert_eq!(
        response
            .get("result")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str),
        Some("pong")
    );
}

#[test]
fn clipboard_round_trip_set_and_get() {
    let _guard = clipboard_test_lock()
        .lock()
        .expect("clipboard test lock poisoned");
    let mut server = ServerHarness::spawn();

    let connection = server
        .send_request(14, "check_connection", None)
        .expect("check_connection failed");
    let uia_connected = connection
        .get("result")
        .and_then(|result| result.get("uia_connected"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !uia_connected {
        eprintln!("Skipping clipboard test because UI Automation is not connected.");
        return;
    }

    let previous_clipboard = get_clipboard_with_retry(&mut server, 15).ok();

    let unique_text = format!(
        "egui-mcp-clipboard-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos()
    );

    set_clipboard_with_retry(&mut server, 30, &unique_text)
        .expect("failed to set clipboard test text");
    let round_trip = get_clipboard_with_retry(&mut server, 45)
        .expect("failed to read clipboard after setting test text");
    assert_eq!(
        round_trip, unique_text,
        "clipboard round-trip mismatch for set/get"
    );

    if let Some(original) = previous_clipboard {
        let _ = set_clipboard_with_retry(&mut server, 60, &original);
    }
}

#[test]
fn check_connection_returns_expected_fields() {
    let mut server = ServerHarness::spawn();
    let response = server
        .send_request(2, "check_connection", None)
        .expect("check_connection failed");

    let result = response.get("result").expect("missing result");
    assert!(result.get("ipc_connected").is_some());
    assert!(result.get("uia_connected").is_some());
    assert!(result.get("uia_window_available").is_some());
    assert!(result.get("status").is_some());
}

#[test]
fn query_tools_reject_invalid_params_with_jsonrpc_invalid_params() {
    let mut server = ServerHarness::spawn();

    let missing_label = server
        .send_request(3, "find_by_label", Some(json!({})))
        .expect("missing label request failed");
    assert_error_code(&missing_label, -32602);

    let empty_label = server
        .send_request(4, "find_by_label_exact", Some(json!({"label": "   "})))
        .expect("empty label request failed");
    assert_error_code(&empty_label, -32602);

    let non_string_role = server
        .send_request(5, "find_by_role", Some(json!({"role": 123})))
        .expect("non-string role request failed");
    assert_error_code(&non_string_role, -32602);

    let empty_element_id = server
        .send_request(6, "get_element", Some(json!({"element_id": ""})))
        .expect("empty element_id request failed");
    assert_error_code(&empty_element_id, -32602);

    let unsupported_role = server
        .send_request(7, "find_by_role", Some(json!({"role": "NotARealRole"})))
        .expect("unsupported role request failed");
    assert_error_code(&unsupported_role, -32000);
    assert_error_message_contains(&unsupported_role, "Unsupported role");
}

#[test]
fn get_ui_tree_method_is_registered() {
    let mut server = ServerHarness::spawn();
    let response = server
        .send_request(7, "get_ui_tree", None)
        .expect("get_ui_tree request failed");

    if let Some(error) = response.get("error") {
        let code = error
            .get("code")
            .and_then(Value::as_i64)
            .expect("missing error.code");
        assert_ne!(
            code, -32601,
            "get_ui_tree should be registered, not method-not-found"
        );
    } else {
        assert!(
            response.get("result").is_some(),
            "expected result for successful get_ui_tree"
        );
    }
}

#[test]
fn priority3_tools_reject_invalid_params() {
    let mut server = ServerHarness::spawn();

    let nearest_missing_x = server
        .send_request(8, "find_nearest_element", Some(json!({ "y": 10 })))
        .expect("find_nearest_element request failed");
    assert_error_code(&nearest_missing_x, -32602);

    let menu_missing_label = server
        .send_request(9, "select_menu_item", Some(json!({})))
        .expect("select_menu_item request failed");
    assert_error_code(&menu_missing_label, -32602);

    let right_click_missing_element = server
        .send_request(10, "right_click_element", Some(json!({})))
        .expect("right_click_element request failed");
    assert_error_code(&right_click_missing_element, -32602);

    let highlight_missing_element = server
        .send_request(11, "highlight_element", Some(json!({})))
        .expect("highlight_element request failed");
    assert_error_code(&highlight_missing_element, -32602);

    let wait_stable_missing_element = server
        .send_request(12, "wait_for_element_stable", Some(json!({})))
        .expect("wait_for_element_stable request failed");
    assert_error_code(&wait_stable_missing_element, -32602);

    let wait_change_missing_element = server
        .send_request(13, "wait_for_value_change", Some(json!({})))
        .expect("wait_for_value_change request failed");
    assert_error_code(&wait_change_missing_element, -32602);
}

#[test]
fn screenshot_compare_and_diff_reject_invalid_params() {
    let mut server = ServerHarness::spawn();

    let missing_compare_params = server
        .send_request(500, "compare_screenshots", Some(json!({})))
        .expect("compare_screenshots request failed");
    assert_error_code(&missing_compare_params, -32602);

    let invalid_threshold = server
        .send_request(
            501,
            "compare_screenshots",
            Some(json!({
                "image_a_base64": png_base64(1, 1, [0, 0, 0, 255]),
                "image_b_base64": png_base64(1, 1, [0, 0, 0, 255]),
                "threshold": 1.5
            })),
        )
        .expect("compare_screenshots invalid threshold request failed");
    assert_error_code(&invalid_threshold, -32602);

    let invalid_highlight_color = server
        .send_request(
            502,
            "diff_screenshots",
            Some(json!({
                "image_a_base64": png_base64(1, 1, [0, 0, 0, 255]),
                "image_b_base64": png_base64(1, 1, [255, 255, 255, 255]),
                "highlight_color": "not-a-color"
            })),
        )
        .expect("diff_screenshots invalid highlight color request failed");
    assert_error_code(&invalid_highlight_color, -32602);
}

#[test]
fn screenshot_compare_and_diff_require_matching_dimensions() {
    let mut server = ServerHarness::spawn();

    let mismatch = server
        .send_request(
            510,
            "compare_screenshots",
            Some(json!({
                "image_a_base64": png_base64(1, 1, [0, 0, 0, 255]),
                "image_b_base64": png_base64(2, 2, [0, 0, 0, 255])
            })),
        )
        .expect("compare_screenshots mismatch request failed");
    assert_error_code(&mismatch, -32000);
    assert_error_message_contains(&mismatch, "dimensions");

    let mismatch_diff = server
        .send_request(
            511,
            "diff_screenshots",
            Some(json!({
                "image_a_base64": png_base64(1, 1, [0, 0, 0, 255]),
                "image_b_base64": png_base64(2, 2, [0, 0, 0, 255])
            })),
        )
        .expect("diff_screenshots mismatch request failed");
    assert_error_code(&mismatch_diff, -32000);
    assert_error_message_contains(&mismatch_diff, "dimensions");
}

#[test]
fn snapshot_save_load_and_diff_round_trip_works_without_live_ipc() {
    let mut server = ServerHarness::spawn();

    let save_a = server
        .send_request(
            520,
            "save_snapshot",
            Some(json!({
                "name": "test-snapshot-a",
                "image_base64": png_base64(4, 4, [10, 10, 10, 255])
            })),
        )
        .expect("save_snapshot A failed");
    assert!(
        save_a.get("error").is_none(),
        "save_snapshot A returned error"
    );
    let snapshot_a_id = save_a
        .get("result")
        .and_then(|value| value.get("snapshot_id"))
        .and_then(Value::as_str)
        .expect("save_snapshot A missing snapshot_id")
        .to_string();

    let save_b = server
        .send_request(
            521,
            "save_snapshot",
            Some(json!({
                "name": "test-snapshot-b",
                "image_base64": png_base64(4, 4, [200, 40, 40, 255])
            })),
        )
        .expect("save_snapshot B failed");
    assert!(
        save_b.get("error").is_none(),
        "save_snapshot B returned error"
    );
    let snapshot_b_id = save_b
        .get("result")
        .and_then(|value| value.get("snapshot_id"))
        .and_then(Value::as_str)
        .expect("save_snapshot B missing snapshot_id")
        .to_string();

    let load_a = server
        .send_request(
            522,
            "load_snapshot",
            Some(json!({
                "snapshot_id": snapshot_a_id
            })),
        )
        .expect("load_snapshot A failed");
    assert!(
        load_a.get("error").is_none(),
        "load_snapshot A returned error"
    );
    assert!(
        load_a
            .get("result")
            .and_then(|value| value.get("image_base64"))
            .and_then(Value::as_str)
            .is_some(),
        "load_snapshot A missing image_base64"
    );

    let diff = server
        .send_request(
            523,
            "diff_snapshots",
            Some(json!({
                "left_snapshot_id": snapshot_a_id,
                "right_snapshot_id": snapshot_b_id
            })),
        )
        .expect("diff_snapshots failed");
    assert!(diff.get("error").is_none(), "diff_snapshots returned error");
    let changed_pixels = diff
        .get("result")
        .and_then(|value| value.get("changed_pixels"))
        .and_then(Value::as_u64)
        .expect("diff_snapshots missing changed_pixels");
    assert!(
        changed_pixels > 0,
        "expected changed pixels for different snapshots"
    );
}

#[test]
#[ignore = "requires a target app running and reachable through UIA + IPC"]
fn priority2_query_tools_succeed_against_live_demo_app() {
    if !live_tests_enabled() {
        eprintln!(
            "Skipping live UI test. Set {}=1 to run this test.",
            LIVE_TESTS_ENV
        );
        return;
    }
    if !has_interactive_session_hint() && !force_live_tests_enabled() {
        eprintln!(
            "Skipping live UI test because no interactive session was detected (SESSIONNAME). \
Set {}=1 to override.",
            FORCE_LIVE_TESTS_ENV
        );
        return;
    }

    let mut server = ServerHarness::spawn();

    let ui_tree = server
        .send_request(100, "get_ui_tree", None)
        .expect("get_ui_tree failed");
    assert!(ui_tree.get("error").is_none(), "get_ui_tree returned error");

    let by_label = server
        .send_request(101, "find_by_label", Some(json!({"label": "Check"})))
        .expect("find_by_label failed");
    assert!(
        by_label.get("error").is_none(),
        "find_by_label returned error"
    );

    let by_label_exact = server
        .send_request(
            102,
            "find_by_label_exact",
            Some(json!({"label": "Checkbox"})),
        )
        .expect("find_by_label_exact failed");
    assert!(
        by_label_exact.get("error").is_none(),
        "find_by_label_exact returned error"
    );

    let by_role = server
        .send_request(103, "find_by_role", Some(json!({"role": "Slider"})))
        .expect("find_by_role failed");
    assert!(
        by_role.get("error").is_none(),
        "find_by_role returned error"
    );

    let element_id = by_label_exact
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .expect("expected non-empty element id from find_by_label_exact");

    let element = server
        .send_request(
            104,
            "get_element",
            Some(json!({ "element_id": element_id })),
        )
        .expect("get_element failed");
    assert!(element.get("error").is_none(), "get_element returned error");
}

#[test]
#[ignore = "requires a target app running and reachable through UIA + IPC"]
fn priority3_tools_succeed_against_live_demo_app() {
    if !live_tests_enabled() {
        eprintln!(
            "Skipping live UI test. Set {}=1 to run this test.",
            LIVE_TESTS_ENV
        );
        return;
    }
    if !has_interactive_session_hint() && !force_live_tests_enabled() {
        eprintln!(
            "Skipping live UI test because no interactive session was detected (SESSIONNAME). \
Set {}=1 to override.",
            FORCE_LIVE_TESTS_ENV
        );
        return;
    }

    let mut server = ServerHarness::spawn();

    let checkbox = server
        .send_request(
            200,
            "find_by_label_exact",
            Some(json!({"label": "Checkbox"})),
        )
        .expect("find_by_label_exact failed");
    assert!(
        checkbox.get("error").is_none(),
        "find_by_label_exact returned error"
    );

    let checkbox_id = checkbox
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .expect("expected non-empty checkbox element id");

    let nearest = server
        .send_request(
            201,
            "find_nearest_element",
            Some(json!({"x": 0, "y": 0, "max_distance": 5000.0})),
        )
        .expect("find_nearest_element failed");
    assert!(
        nearest.get("error").is_none(),
        "find_nearest_element returned error"
    );

    let highlight = server
        .send_request(
            202,
            "highlight_element",
            Some(json!({"element_id": checkbox_id, "delay_ms": 20})),
        )
        .expect("highlight_element failed");
    assert!(
        highlight.get("error").is_none(),
        "highlight_element returned error"
    );

    let flash = server
        .send_request(
            203,
            "flash_element",
            Some(json!({"element_id": checkbox_id, "flashes": 1, "delay_ms": 20})),
        )
        .expect("flash_element failed");
    assert!(flash.get("error").is_none(), "flash_element returned error");

    let actions = server
        .send_request(
            204,
            "find_by_label_exact",
            Some(json!({"label": "Actions"})),
        )
        .expect("find_by_label_exact for Actions failed");
    assert!(
        actions.get("error").is_none(),
        "find_by_label_exact for Actions returned error"
    );
    let actions_id = actions
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .expect("expected non-empty Actions element id");

    let opened_menu = server
        .send_request(
            205,
            "activate_element",
            Some(json!({"element_id": actions_id})),
        )
        .expect("activate_element for Actions failed");
    assert!(
        opened_menu.get("error").is_none(),
        "activate_element for Actions returned error"
    );

    let selected_menu_item = server
        .send_request(
            206,
            "select_menu_item",
            Some(json!({"label": "Action Alpha", "exact": true})),
        )
        .expect("select_menu_item failed");
    assert!(
        selected_menu_item.get("error").is_none(),
        "select_menu_item returned error"
    );

    let action_state = server
        .send_request(
            207,
            "find_by_label_exact",
            Some(json!({"label": "Menu action: Action Alpha"})),
        )
        .expect("find_by_label_exact for menu action state failed");
    assert!(
        action_state
            .get("result")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty()),
        "expected menu action label to reflect selected item"
    );

    let right_click = server
        .send_request(
            208,
            "right_click_element",
            Some(json!({"element_id": checkbox_id})),
        )
        .expect("right_click_element failed");
    assert!(
        right_click.get("error").is_none(),
        "right_click_element returned error"
    );

    let context_menu = server
        .send_request(
            209,
            "open_context_menu",
            Some(json!({"element_id": checkbox_id})),
        )
        .expect("open_context_menu failed");
    assert!(
        context_menu.get("error").is_none(),
        "open_context_menu returned error"
    );

    let stable = server
        .send_request(
            210,
            "wait_for_element_stable",
            Some(json!({
                "element_id": checkbox_id,
                "stable_ms": 50,
                "timeout_ms": 2000,
                "poll_interval_ms": 50
            })),
        )
        .expect("wait_for_element_stable failed");
    assert!(
        stable.get("error").is_none(),
        "wait_for_element_stable returned error"
    );

    let current_value = server
        .send_request(
            211,
            "get_element_value",
            Some(json!({"element_id": checkbox_id})),
        )
        .expect("get_element_value failed");
    let initial_value = current_value
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_str)
        .expect("missing value from get_element_value");

    let toggled = server
        .send_request(
            212,
            "toggle_checkbox",
            Some(json!({"element_id": checkbox_id})),
        )
        .expect("toggle_checkbox failed");
    assert!(
        toggled.get("error").is_none(),
        "toggle_checkbox returned error"
    );

    let changed = server
        .send_request(
            213,
            "wait_for_value_change",
            Some(json!({
                "element_id": checkbox_id,
                "initial_value": initial_value,
                "timeout_ms": 2000,
                "poll_interval_ms": 50
            })),
        )
        .expect("wait_for_value_change failed");
    assert!(
        changed.get("error").is_none(),
        "wait_for_value_change returned error"
    );
}

#[test]
#[ignore = "requires a target app running and reachable through UIA + IPC"]
fn priority3_negative_paths_surface_timeout_and_unsupported_pattern_errors() {
    if !live_tests_enabled() {
        eprintln!(
            "Skipping live UI test. Set {}=1 to run this test.",
            LIVE_TESTS_ENV
        );
        return;
    }
    if !has_interactive_session_hint() && !force_live_tests_enabled() {
        eprintln!(
            "Skipping live UI test because no interactive session was detected (SESSIONNAME). \
Set {}=1 to override.",
            FORCE_LIVE_TESTS_ENV
        );
        return;
    }

    let mut server = ServerHarness::spawn();

    let checkbox = server
        .send_request(
            300,
            "find_by_label_exact",
            Some(json!({"label": "Checkbox"})),
        )
        .expect("find_by_label_exact failed");
    assert!(
        checkbox.get("error").is_none(),
        "find_by_label_exact returned error"
    );

    let checkbox_id = checkbox
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .expect("expected non-empty checkbox element id");

    let unsupported_pattern = server
        .send_request(
            301,
            "set_element_value",
            Some(json!({
                "element_id": checkbox_id,
                "value": "123"
            })),
        )
        .expect("set_element_value request failed");
    assert_error_code(&unsupported_pattern, -32000);
    assert_error_message_contains(&unsupported_pattern, "does not support value setting");

    let current_value = server
        .send_request(
            302,
            "get_element_value",
            Some(json!({"element_id": checkbox_id})),
        )
        .expect("get_element_value failed");
    let baseline = current_value
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_str)
        .expect("missing value from get_element_value");

    let timeout_response = server
        .send_request(
            303,
            "wait_for_value_change",
            Some(json!({
                "element_id": checkbox_id,
                "initial_value": baseline,
                "timeout_ms": 150,
                "poll_interval_ms": 25
            })),
        )
        .expect("wait_for_value_change request failed");
    assert_error_code(&timeout_response, -32000);
    assert_error_message_contains(&timeout_response, "did not change");
}

// --- MCP protocol conformance -------------------------------------------------

#[test]
fn initialize_returns_protocol_version_and_server_info() {
    let mut server = ServerHarness::spawn();

    let response = server
        .send_request(
            700,
            "initialize",
            Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "integration-test", "version": "0.0.0" }
            })),
        )
        .expect("initialize failed");

    assert!(
        response.get("error").is_none(),
        "initialize returned an error: {response}"
    );
    let result = response.get("result").expect("initialize missing result");

    assert_eq!(
        result.get("protocolVersion").and_then(Value::as_str),
        Some("2024-11-05"),
        "server should echo a protocol version it supports"
    );
    assert!(
        result
            .get("capabilities")
            .and_then(|caps| caps.get("tools"))
            .is_some(),
        "server must advertise the tools capability"
    );
    assert_eq!(
        result
            .get("serverInfo")
            .and_then(|info| info.get("name"))
            .and_then(Value::as_str),
        Some("egui-mcp-server-win")
    );
    assert!(
        result
            .get("serverInfo")
            .and_then(|info| info.get("version"))
            .and_then(Value::as_str)
            .is_some_and(|version| !version.is_empty()),
        "serverInfo.version must be populated"
    );
}

#[test]
fn initialize_falls_back_for_unknown_protocol_version() {
    let mut server = ServerHarness::spawn();

    let response = server
        .send_request(
            701,
            "initialize",
            Some(json!({"protocolVersion": "1999-01-01"})),
        )
        .expect("initialize failed");

    assert_eq!(
        response["result"]["protocolVersion"].as_str(),
        Some("2024-11-05"),
        "unknown versions should degrade to the server default rather than fail"
    );
}

#[test]
fn notifications_are_not_answered_and_do_not_kill_the_server() {
    let mut server = ServerHarness::spawn();

    server.send_notification("notifications/initialized", None);
    server.send_notification("notifications/cancelled", Some(json!({"requestId": 1})));

    // The server must still answer normal requests afterwards.
    let response = server
        .send_request(702, "ping", None)
        .expect("server stopped responding after receiving notifications");
    assert_eq!(response["result"]["status"].as_str(), Some("pong"));
}

#[test]
fn tools_list_advertises_tools_with_object_schemas() {
    let mut server = ServerHarness::spawn();

    let response = server
        .send_request(703, "tools/list", None)
        .expect("tools/list failed");
    assert!(
        response.get("error").is_none(),
        "tools/list returned an error: {response}"
    );

    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools/list must return an array of tools");
    assert!(
        tools.len() >= 50,
        "expected the full tool surface, got {}",
        tools.len()
    );

    for tool in tools {
        let name = tool["name"].as_str().expect("tool missing name");
        assert!(
            tool["description"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "tool '{name}' has no description"
        );
        assert_eq!(
            tool["inputSchema"]["type"].as_str(),
            Some("object"),
            "tool '{name}' must declare an object schema"
        );
    }

    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for expected in [
        "ping",
        "get_ui_tree",
        "find_by_label",
        "click_at",
        "take_screenshot",
        "set_target_window",
    ] {
        assert!(
            names.contains(&expected),
            "tools/list is missing '{expected}'"
        );
    }
}

#[test]
fn tools_call_wraps_results_in_content_blocks() {
    let mut server = ServerHarness::spawn();

    let response = server
        .send_request(704, "tools/call", Some(json!({"name": "ping"})))
        .expect("tools/call failed");
    assert!(
        response.get("error").is_none(),
        "tools/call returned an error: {response}"
    );

    let result = &response["result"];
    assert_eq!(result["isError"].as_bool(), Some(false));

    let content = result["content"]
        .as_array()
        .expect("tools/call result must carry content blocks");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"].as_str(), Some("text"));
    assert!(
        content[0]["text"].as_str().unwrap().contains("pong"),
        "content should carry the tool's JSON result"
    );
}

#[test]
fn tools_call_rejects_unknown_tool_with_method_not_found() {
    let mut server = ServerHarness::spawn();

    let response = server
        .send_request(705, "tools/call", Some(json!({"name": "no_such_tool"})))
        .expect("tools/call failed");

    assert_error_code(&response, -32601);
}

#[test]
fn tools_call_requires_a_tool_name() {
    let mut server = ServerHarness::spawn();

    let missing = server
        .send_request(706, "tools/call", Some(json!({})))
        .expect("tools/call failed");
    assert_error_code(&missing, -32602);

    let bad_arguments = server
        .send_request(
            707,
            "tools/call",
            Some(json!({"name": "ping", "arguments": "not-an-object"})),
        )
        .expect("tools/call failed");
    assert_error_code(&bad_arguments, -32602);
}

#[test]
fn tools_call_surfaces_tool_failures_as_is_error() {
    let mut server = ServerHarness::spawn();

    // Missing the required `label` argument fails inside the tool, which MCP
    // reports as a successful call carrying isError rather than a JSON-RPC error.
    let response = server
        .send_request(
            708,
            "tools/call",
            Some(json!({"name": "find_by_label", "arguments": {}})),
        )
        .expect("tools/call failed");

    assert!(
        response.get("error").is_none(),
        "tool-level failures belong in the result, not a JSON-RPC error: {response}"
    );
    assert_eq!(response["result"]["isError"].as_bool(), Some(true));
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("label"),
        "error content should name the missing parameter"
    );
}

#[test]
fn every_advertised_tool_is_dispatchable() {
    let mut server = ServerHarness::spawn();

    // A blind sweep would drive real input if an app were attached, so only run
    // it when nothing is reachable.
    if !server.is_isolated() {
        eprintln!("Skipping tool sweep because a live target app is reachable.");
        return;
    }

    let listed = server
        .send_request(709, "tools/list", None)
        .expect("tools/list failed");
    let names: Vec<String> = listed["result"]["tools"]
        .as_array()
        .expect("tools/list must return an array")
        .iter()
        .filter_map(|tool| tool["name"].as_str().map(str::to_string))
        .collect();
    assert!(!names.is_empty(), "tools/list returned nothing");

    for (offset, name) in names.into_iter().enumerate() {
        let response = server
            .send_request(
                7_100 + offset as i64,
                "tools/call",
                Some(json!({"name": name, "arguments": {}})),
            )
            .unwrap_or_else(|err| panic!("tools/call for '{name}' failed: {err}"));

        let code = response
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64);
        assert_ne!(
            code,
            Some(-32601),
            "tool '{name}' is advertised by tools/list but is not dispatchable"
        );
    }
}

// --- Target window configuration ---------------------------------------------

#[test]
fn target_window_is_reported_and_configurable() {
    let mut server = ServerHarness::spawn();

    let initial = server
        .send_request(720, "get_target_window", None)
        .expect("get_target_window failed");
    let original = initial["result"]["target_window"]
        .as_str()
        .expect("get_target_window must report a title")
        .to_string();
    assert!(!original.is_empty(), "target window must not be empty");

    let updated = server
        .send_request(
            721,
            "set_target_window",
            Some(json!({"title": "Some Other Window"})),
        )
        .expect("set_target_window failed");
    assert!(
        updated.get("error").is_none(),
        "set_target_window returned an error: {updated}"
    );
    assert_eq!(
        updated["result"]["target_window"].as_str(),
        Some("Some Other Window")
    );
    assert_eq!(
        updated["result"]["previous_target_window"].as_str(),
        Some(original.as_str())
    );

    // The change must stick and be visible to check_connection.
    let after = server
        .send_request(722, "check_connection", None)
        .expect("check_connection failed");
    assert_eq!(
        after["result"]["target_window"].as_str(),
        Some("Some Other Window")
    );
}

#[test]
fn set_target_window_rejects_empty_titles() {
    let mut server = ServerHarness::spawn();

    for bad in [json!({}), json!({"title": ""}), json!({"title": "   "})] {
        let response = server
            .send_request(723, "set_target_window", Some(bad.clone()))
            .expect("set_target_window request failed");
        assert_error_code(&response, -32602);
    }
}

// --- Transport hygiene --------------------------------------------------------

#[test]
fn stdout_carries_only_json_lines() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_egui-mcp-server-win"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn server");

    let mut stdin = child.stdin.take().expect("missing stdin");
    let stdout = child.stdout.take().expect("missing stdout");
    let stderr = child.stderr.take().expect("missing stderr");
    let _drain = spawn_stderr_drain(stderr);

    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 1, "method": "ping"})
    )
    .and_then(|_| stdin.flush())
    .expect("failed to write ping");

    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("failed to read stdout");

    let _ = child.kill();
    let _ = child.wait();

    let trimmed = line.trim();
    assert!(
        !trimmed.is_empty(),
        "server produced no stdout before responding"
    );
    serde_json::from_str::<Value>(trimmed).unwrap_or_else(|err| {
        panic!("stdout must carry only JSON-RPC lines, got {trimmed:?}: {err}")
    });
}

#[test]
fn check_connection_reports_disconnected_when_ipc_server_absent() {
    let mut server = ServerHarness::spawn();
    let response = server
        .send_request(801, "check_connection", None)
        .expect("check_connection request failed");

    assert_eq!(response["result"]["ipc_connected"], false);
    assert_eq!(response["result"]["status"], "partial");
}

#[test]
fn input_tools_return_clean_errors_when_ipc_server_absent() {
    let mut server = ServerHarness::spawn();

    let right_click_response = server
        .send_request(
            802,
            "tools/call",
            Some(json!({
                "name": "right_click_at",
                "arguments": {"x": 100, "y": 100}
            })),
        )
        .expect("right_click_at call failed");

    assert_eq!(right_click_response["result"]["isError"], true);

    let drag_response = server
        .send_request(
            803,
            "tools/call",
            Some(json!({
                "name": "drag",
                "arguments": {"x1": 100, "y1": 100, "x2": 200, "y2": 200}
            })),
        )
        .expect("drag call failed");

    assert_eq!(drag_response["result"]["isError"], true);
}
