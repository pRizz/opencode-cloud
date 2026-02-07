//! Docker resource naming for optional sandbox profile isolation.
//!
//! Legacy behavior uses shared resource names (`opencode-cloud-sandbox`, `latest`, etc.).
//! When `OPENCODE_SANDBOX_INSTANCE` is set to a valid instance ID, names are derived with
//! profile-specific suffixes/tags so concurrent worktrees can run independently.

use super::container::CONTAINER_NAME;
use super::dockerfile::IMAGE_TAG_DEFAULT;
use super::volume::{
    VOLUME_CACHE, VOLUME_CONFIG, VOLUME_PROJECTS, VOLUME_SESSION, VOLUME_STATE, VOLUME_USERS,
};
use std::env;

/// Environment variable carrying the active sandbox instance id.
pub const SANDBOX_INSTANCE_ENV: &str = "OPENCODE_SANDBOX_INSTANCE";

/// Container and volume label key identifying the active instance.
pub const INSTANCE_LABEL_KEY: &str = "opencode-cloud.instance";

/// Legacy rollback tag for shared mode.
const PREVIOUS_TAG_DEFAULT: &str = "previous";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerResourceNames {
    pub instance_id: Option<String>,
    pub suffix: Option<String>,
    pub container_name: String,
    pub hostname: String,
    pub image_tag: String,
    pub previous_image_tag: String,
    pub volume_session: String,
    pub volume_state: String,
    pub volume_cache: String,
    pub volume_projects: String,
    pub volume_config: String,
    pub volume_users: String,
    pub image_state_file: String,
}

impl DockerResourceNames {
    pub fn volume_names(&self) -> [&str; 6] {
        [
            &self.volume_session,
            &self.volume_state,
            &self.volume_cache,
            &self.volume_projects,
            &self.volume_config,
            &self.volume_users,
        ]
    }
}

/// Resolve active resource names from `OPENCODE_SANDBOX_INSTANCE`.
pub fn active_resource_names() -> DockerResourceNames {
    resource_names_for_instance(env_instance_id().as_deref())
}

/// Resolve resource names for an optional instance id.
pub fn resource_names_for_instance(instance_id: Option<&str>) -> DockerResourceNames {
    if let Some(instance_id) = instance_id {
        let suffix = format!("-{instance_id}");
        // Keep default names untouched for backward compatibility; profile mode only appends.
        DockerResourceNames {
            instance_id: Some(instance_id.to_string()),
            suffix: Some(suffix.clone()),
            container_name: format!("{CONTAINER_NAME}{suffix}"),
            hostname: format!("{CONTAINER_NAME}{suffix}"),
            image_tag: format!("instance-{instance_id}"),
            previous_image_tag: format!("instance-{instance_id}-previous"),
            volume_session: format!("{VOLUME_SESSION}{suffix}"),
            volume_state: format!("{VOLUME_STATE}{suffix}"),
            volume_cache: format!("{VOLUME_CACHE}{suffix}"),
            volume_projects: format!("{VOLUME_PROJECTS}{suffix}"),
            volume_config: format!("{VOLUME_CONFIG}{suffix}"),
            volume_users: format!("{VOLUME_USERS}{suffix}"),
            image_state_file: format!("image-state-{instance_id}.json"),
        }
    } else {
        DockerResourceNames {
            instance_id: None,
            suffix: None,
            container_name: CONTAINER_NAME.to_string(),
            hostname: CONTAINER_NAME.to_string(),
            image_tag: IMAGE_TAG_DEFAULT.to_string(),
            previous_image_tag: PREVIOUS_TAG_DEFAULT.to_string(),
            volume_session: VOLUME_SESSION.to_string(),
            volume_state: VOLUME_STATE.to_string(),
            volume_cache: VOLUME_CACHE.to_string(),
            volume_projects: VOLUME_PROJECTS.to_string(),
            volume_config: VOLUME_CONFIG.to_string(),
            volume_users: VOLUME_USERS.to_string(),
            image_state_file: "image-state.json".to_string(),
        }
    }
}

/// Remap legacy container name to active profile container name.
pub fn remap_container_name(name: &str) -> String {
    if name == CONTAINER_NAME {
        return active_resource_names().container_name;
    }
    name.to_string()
}

/// Remap legacy image tags to active profile tags.
pub fn remap_image_tag(tag: &str) -> String {
    let names = active_resource_names();
    if tag == IMAGE_TAG_DEFAULT {
        return names.image_tag;
    }
    if tag == PREVIOUS_TAG_DEFAULT {
        return names.previous_image_tag;
    }
    tag.to_string()
}

/// Read and validate sandbox instance from environment.
pub fn env_instance_id() -> Option<String> {
    let raw = env::var(SANDBOX_INSTANCE_ENV).ok()?;
    let trimmed = raw.trim().to_ascii_lowercase();
    if is_valid_instance_id(&trimmed) {
        Some(trimmed)
    } else {
        None
    }
}

fn is_valid_instance_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 32 {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_names_remain_unchanged() {
        let names = resource_names_for_instance(None);
        assert_eq!(names.container_name, CONTAINER_NAME);
        assert_eq!(names.image_tag, IMAGE_TAG_DEFAULT);
        assert_eq!(names.volume_users, VOLUME_USERS);
        assert_eq!(names.image_state_file, "image-state.json");
        assert!(names.instance_id.is_none());
    }

    #[test]
    fn isolated_names_include_suffixes() {
        let names = resource_names_for_instance(Some("foo"));
        assert_eq!(names.container_name, "opencode-cloud-sandbox-foo");
        assert_eq!(names.image_tag, "instance-foo");
        assert_eq!(names.previous_image_tag, "instance-foo-previous");
        assert_eq!(names.volume_users, "opencode-users-foo");
        assert_eq!(names.image_state_file, "image-state-foo.json");
        assert_eq!(names.instance_id.as_deref(), Some("foo"));
    }

    #[test]
    fn env_instance_id_rejects_invalid_values() {
        assert!(is_valid_instance_id("foo-123"));
        assert!(!is_valid_instance_id(""));
        assert!(!is_valid_instance_id("-foo"));
        assert!(!is_valid_instance_id("foo_bar"));
        assert!(!is_valid_instance_id("Foo"));
    }
}
