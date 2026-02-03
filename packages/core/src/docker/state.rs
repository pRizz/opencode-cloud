//! Image state tracking for provenance information
//!
//! Tracks where the current Docker image came from (prebuilt or built)
//! and which registry it was pulled from.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Image provenance state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageState {
    /// Image version (e.g., "1.0.12")
    pub version: String,
    /// Source: "prebuilt" or "build"
    pub source: String,
    /// Registry if prebuilt: "ghcr.io" or "docker.io", None for build
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// When the image was acquired (ISO8601)
    pub acquired_at: String,
}

impl ImageState {
    /// Create a new ImageState for a prebuilt image
    pub fn prebuilt(version: &str, registry: &str) -> Self {
        Self {
            version: version.to_string(),
            source: "prebuilt".to_string(),
            registry: Some(registry.to_string()),
            acquired_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create a new ImageState for a locally built image
    pub fn built(version: &str) -> Self {
        Self {
            version: version.to_string(),
            source: "build".to_string(),
            registry: None,
            acquired_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Get the path to the image state file
pub fn get_state_path() -> Option<PathBuf> {
    crate::config::paths::get_data_dir().map(|p| p.join("image-state.json"))
}

/// Save image state to disk
pub fn save_state(state: &ImageState) -> anyhow::Result<()> {
    let path = get_state_path().ok_or_else(|| anyhow::anyhow!("Could not determine state path"))?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Load image state from disk
pub fn load_state() -> Option<ImageState> {
    let path = get_state_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Clear image state (e.g., after image removal)
pub fn clear_state() -> anyhow::Result<()> {
    if let Some(path) = get_state_path()
        && path.exists()
    {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_state_prebuilt() {
        let state = ImageState::prebuilt("1.0.12", "ghcr.io");
        assert_eq!(state.version, "1.0.12");
        assert_eq!(state.source, "prebuilt");
        assert_eq!(state.registry, Some("ghcr.io".to_string()));
        assert!(!state.acquired_at.is_empty());
    }

    #[test]
    fn test_image_state_built() {
        let state = ImageState::built("1.0.12");
        assert_eq!(state.version, "1.0.12");
        assert_eq!(state.source, "build");
        assert!(state.registry.is_none());
    }

    #[test]
    fn test_image_state_serialize_deserialize() {
        let state = ImageState::prebuilt("1.0.12", "docker.io");
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ImageState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.version, parsed.version);
        assert_eq!(state.source, parsed.source);
        assert_eq!(state.registry, parsed.registry);
    }

    #[test]
    fn test_get_state_path() {
        let path = get_state_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("image-state.json"));
    }
}
