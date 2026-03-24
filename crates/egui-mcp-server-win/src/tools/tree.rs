#![allow(dead_code)]

use crate::uia_client::UiaClient;
use anyhow::Result;
use serde_json::Value;

pub async fn get_ui_tree(_uia_client: &UiaClient) -> Result<Value> {
    Ok(serde_json::json!({"message": "UI tree not implemented"}))
}

pub async fn find_by_label(_uia_client: &UiaClient, _name: &str) -> Result<Value> {
    Ok(serde_json::json!([]))
}

pub async fn find_by_role(_uia_client: &UiaClient, _role: &str) -> Result<Value> {
    Ok(serde_json::json!([]))
}

pub async fn get_element(_uia_client: &UiaClient, _automation_id: &str) -> Result<Value> {
    Ok(serde_json::json!({"message": "Not found"}))
}
