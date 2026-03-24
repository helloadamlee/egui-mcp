use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::IpcClient;
use crate::UiaClient;

pub mod input;
pub mod interaction;
pub mod tree;

#[allow(dead_code)]
pub async fn register_tools(
    _server: (),
    _uia_client: Arc<Mutex<UiaClient>>,
    _ipc_client: Arc<Mutex<IpcClient>>,
) -> Result<()> {
    Ok(())
}

#[allow(dead_code)]
pub async fn get_ui_tree(_uia_client: &Arc<Mutex<UiaClient>>) -> Result<Value> {
    Ok(serde_json::json!({"message": "UI tree not implemented"}))
}

#[allow(dead_code)]
pub async fn find_by_label(_uia_client: &Arc<Mutex<UiaClient>>, _name: &str) -> Result<Value> {
    Ok(serde_json::json!([]))
}

#[allow(dead_code)]
pub async fn click_element(
    _uia_client: &Arc<Mutex<UiaClient>>,
    _automation_id: &str,
) -> Result<()> {
    Ok(())
}

#[allow(dead_code)]
pub async fn click_at(_ipc_client: &Arc<Mutex<IpcClient>>, _x: i32, _y: i32) -> Result<()> {
    Ok(())
}

#[allow(dead_code)]
pub async fn take_screenshot(_ipc_client: &Arc<Mutex<IpcClient>>) -> Result<String> {
    Ok(String::new())
}
