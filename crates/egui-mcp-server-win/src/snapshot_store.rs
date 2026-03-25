use crate::screenshot_core::{decode_png_base64, encode_png_base64};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub id: String,
    pub name: Option<String>,
    pub label: Option<String>,
    pub created_at_epoch_ms: u128,
    pub width: u32,
    pub height: u32,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct LoadedSnapshot {
    pub metadata: SnapshotMetadata,
    pub image: RgbaImage,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SnapshotIndex {
    snapshots: Vec<SnapshotMetadata>,
}

pub struct SnapshotStore {
    images_dir: PathBuf,
    index_path: PathBuf,
}

impl SnapshotStore {
    pub fn new() -> Result<Self, String> {
        Self::new_with_root(default_snapshot_root())
    }

    pub fn new_with_root(root_dir: impl Into<PathBuf>) -> Result<Self, String> {
        let root_dir = root_dir.into();
        let images_dir = root_dir.join("images");
        fs::create_dir_all(&images_dir)
            .map_err(|err| format!("Failed to create snapshot images directory: {}", err))?;

        let index_path = root_dir.join("index.json");
        if !index_path.exists() {
            let empty_index = SnapshotIndex::default();
            write_json_atomic(&index_path, &empty_index)?;
        }

        Ok(Self {
            images_dir,
            index_path,
        })
    }

    pub fn save_snapshot(
        &self,
        image_base64: &str,
        name: Option<&str>,
        label: Option<&str>,
    ) -> Result<SnapshotMetadata, String> {
        let image = decode_png_base64(image_base64)?;
        let normalized_png_base64 = encode_png_base64(&image)?;
        let normalized_image = decode_png_base64(&normalized_png_base64)?;

        let snapshot_id = generate_snapshot_id();
        let file_name = format!("{}.png", snapshot_id);
        let image_path = self.images_dir.join(&file_name);

        save_png_atomic(&image_path, &normalized_image)?;

        let metadata = SnapshotMetadata {
            id: snapshot_id,
            name: normalize_optional_string(name),
            label: normalize_optional_string(label),
            created_at_epoch_ms: now_epoch_ms(),
            width: normalized_image.width(),
            height: normalized_image.height(),
            file_name,
        };

        let mut index = self.load_index()?;
        index.snapshots.push(metadata.clone());
        self.write_index(&index)?;

        Ok(metadata)
    }

    pub fn load_snapshot(
        &self,
        snapshot_id: Option<&str>,
        name: Option<&str>,
    ) -> Result<LoadedSnapshot, String> {
        let metadata = self.find_snapshot(snapshot_id, name)?;
        let image_path = self.images_dir.join(&metadata.file_name);

        let image = image::open(&image_path)
            .map_err(|err| {
                format!(
                    "Failed to open snapshot image '{}': {}",
                    image_path.display(),
                    err
                )
            })?
            .to_rgba8();

        Ok(LoadedSnapshot { metadata, image })
    }

    fn find_snapshot(
        &self,
        snapshot_id: Option<&str>,
        name: Option<&str>,
    ) -> Result<SnapshotMetadata, String> {
        let id = normalize_optional_string(snapshot_id);
        let target_name = normalize_optional_string(name);

        if id.is_none() && target_name.is_none() {
            return Err("Expected either 'snapshot_id' or 'name'".to_string());
        }

        let index = self.load_index()?;

        if let Some(id) = id {
            return index
                .snapshots
                .iter()
                .find(|entry| entry.id == id)
                .cloned()
                .ok_or_else(|| format!("Snapshot id '{}' was not found", id));
        }

        let name = target_name.expect("checked above");
        index
            .snapshots
            .iter()
            .rev()
            .find(|entry| entry.name.as_deref() == Some(name.as_str()))
            .cloned()
            .ok_or_else(|| format!("Snapshot name '{}' was not found", name))
    }

    fn load_index(&self) -> Result<SnapshotIndex, String> {
        let content = fs::read_to_string(&self.index_path)
            .map_err(|err| format!("Failed to read snapshot index: {}", err))?;

        serde_json::from_str(&content)
            .map_err(|err| format!("Failed to parse snapshot index JSON: {}", err))
    }

    fn write_index(&self, index: &SnapshotIndex) -> Result<(), String> {
        write_json_atomic(&self.index_path, index)
    }
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn save_png_atomic(path: &Path, image: &RgbaImage) -> Result<(), String> {
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|err| format!("Failed to encode PNG for '{}': {}", path.display(), err))?;

    write_bytes_atomic(path, &bytes)
}

fn write_json_atomic<T: Serialize>(path: &Path, payload: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(payload)
        .map_err(|err| format!("Failed to serialize snapshot JSON: {}", err))?;
    write_bytes_atomic(path, &bytes)
}

fn write_bytes_atomic(path: &Path, payload: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create '{}' for atomic write: {}",
                parent.display(),
                err
            )
        })?;
    }

    let temp_name = format!(
        "{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("snapshot"),
        std::process::id()
    );
    let temp_path = path.with_file_name(temp_name);

    fs::write(&temp_path, payload).map_err(|err| {
        format!(
            "Failed to write temp file '{}': {}",
            temp_path.display(),
            err
        )
    })?;

    if path.exists() {
        fs::remove_file(path).map_err(|err| {
            format!(
                "Failed to replace existing file '{}': {}",
                path.display(),
                err
            )
        })?;
    }

    fs::rename(&temp_path, path).map_err(|err| {
        format!(
            "Failed to atomically rename '{}' to '{}': {}",
            temp_path.display(),
            path.display(),
            err
        )
    })?;

    Ok(())
}

fn default_snapshot_root() -> PathBuf {
    if let Ok(configured) = std::env::var("EGUI_MCP_SNAPSHOT_DIR") {
        let configured = configured.trim();
        if !configured.is_empty() {
            return PathBuf::from(configured);
        }
    }

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("egui-mcp")
            .join("snapshots");
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".egui-mcp")
        .join("snapshots")
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn generate_snapshot_id() -> String {
    let now = now_epoch_ms();
    let counter = SNAPSHOT_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("snap-{}-{}", now, counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "egui-mcp-snapshot-store-test-{}-{}",
            name,
            now_epoch_ms()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn save_and_load_snapshot_round_trip() {
        let dir = temp_dir("round-trip");
        let store = SnapshotStore::new_with_root(&dir).expect("store");
        let image = RgbaImage::from_fn(2, 2, |_x, _y| Rgba([12, 34, 56, 255]));
        let encoded = encode_png_base64(&image).expect("encode");

        let saved = store
            .save_snapshot(&encoded, Some("login-screen"), Some("before-click"))
            .expect("save");
        let loaded = store
            .load_snapshot(Some(saved.id.as_str()), None)
            .expect("load");

        assert_eq!(loaded.metadata.width, 2);
        assert_eq!(loaded.metadata.height, 2);
        assert_eq!(loaded.metadata.name.as_deref(), Some("login-screen"));
        assert_eq!(loaded.image, image);

        let _ = fs::remove_dir_all(&dir);
    }
}
