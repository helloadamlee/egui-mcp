#![allow(dead_code)]

use crate::uia_client::UiaClient;
use anyhow::Result;
use serde_json::Value;

pub async fn click_element(_uia_client: &UiaClient, _automation_id: &str) -> Result<()> {
    Ok(())
}

pub async fn focus_element(_uia_client: &UiaClient, _automation_id: &str) -> Result<()> {
    Ok(())
}

pub async fn get_bounds(_uia_client: &UiaClient, _automation_id: &str) -> Result<Value> {
    Ok(serde_json::json!({"x": 0, "y": 0, "width": 0, "height": 0}))
}

pub async fn is_visible(_uia_client: &UiaClient, _automation_id: &str) -> Result<Value> {
    Ok(serde_json::json!({"visible": true}))
}

pub async fn is_enabled(_uia_client: &UiaClient, _automation_id: &str) -> Result<Value> {
    Ok(serde_json::json!({"enabled": true}))
}

pub async fn is_focused(_uia_client: &UiaClient, _automation_id: &str) -> Result<Value> {
    Ok(serde_json::json!({"focused": false}))
}
