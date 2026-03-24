#![allow(dead_code)]

use crate::ipc::IpcClient;
use anyhow::Result;

pub async fn click_at(_ipc_client: &mut IpcClient, _x: i32, _y: i32) -> Result<()> {
    Ok(())
}

pub async fn double_click(_ipc_client: &mut IpcClient, _x: i32, _y: i32) -> Result<()> {
    Ok(())
}

pub async fn hover(_ipc_client: &mut IpcClient, _x: i32, _y: i32) -> Result<()> {
    Ok(())
}

pub async fn drag(
    _ipc_client: &mut IpcClient,
    _x1: i32,
    _y1: i32,
    _x2: i32,
    _y2: i32,
) -> Result<()> {
    Ok(())
}

pub async fn keyboard_input(_ipc_client: &mut IpcClient, _key: &str) -> Result<()> {
    Ok(())
}

pub async fn scroll(_ipc_client: &mut IpcClient, _delta_x: i32, _delta_y: i32) -> Result<()> {
    Ok(())
}
