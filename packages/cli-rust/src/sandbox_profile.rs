use anyhow::{Result, anyhow};
use opencode_cloud_core::docker::SANDBOX_INSTANCE_ENV;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxProfile {
    pub instance_id: Option<String>,
    pub suffix: Option<String>,
}

impl SandboxProfile {
    pub fn shared() -> Self {
        Self {
            instance_id: None,
            suffix: None,
        }
    }

    pub fn isolated(instance_id: String) -> Self {
        Self {
            suffix: Some(format!("-{instance_id}")),
            instance_id: Some(instance_id),
        }
    }
}

pub fn resolve_sandbox_profile(arg_value: Option<&str>) -> Result<SandboxProfile> {
    let raw = arg_value
        .map(str::to_string)
        .or_else(|| std::env::var(SANDBOX_INSTANCE_ENV).ok());
    let Some(raw) = raw else {
        return Ok(SandboxProfile::shared());
    };

    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(anyhow!(
            "--sandbox-instance cannot be empty; use a-z, 0-9, and '-' only"
        ));
    }

    if normalized == "auto" {
        return Ok(SandboxProfile::isolated(derive_auto_instance_id()?));
    }

    let instance_id = normalize_manual_instance_id(&normalized)?;
    Ok(SandboxProfile::isolated(instance_id))
}

pub fn apply_active_profile_env(profile: &SandboxProfile) {
    if let Some(instance_id) = profile.instance_id.as_deref() {
        // SAFETY: This binary is single-process CLI execution and we set a process-local env var
        // before spawning worker runtimes/threads, so this is safe under Rust 2024 env rules.
        unsafe { std::env::set_var(SANDBOX_INSTANCE_ENV, instance_id) };
    } else {
        // SAFETY: Same rationale as set_var above.
        unsafe { std::env::remove_var(SANDBOX_INSTANCE_ENV) };
    }
}

fn normalize_manual_instance_id(value: &str) -> Result<String> {
    if !is_valid_instance_id(value) {
        return Err(anyhow!(
            "Invalid sandbox instance '{value}'. Expected [a-z0-9][a-z0-9-]{{0,31}}"
        ));
    }
    Ok(value.to_string())
}

fn derive_auto_instance_id() -> Result<String> {
    let root = resolve_worktree_root()?;
    let canonical = root.canonicalize().map_err(|err| {
        anyhow!(
            "Failed to canonicalize worktree root {}: {err}",
            root.display()
        )
    })?;
    let path_str = canonical.to_string_lossy();
    let hash = compute_stable_fnv1a_64_hash(path_str.as_bytes());

    // Use a deterministic hash-based id to keep names filesystem-safe and avoid collisions
    // between similarly named worktree directories.
    Ok(format!("wt-{hash:012x}"))
}

fn resolve_worktree_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|err| anyhow!("Failed to run git rev-parse for --sandbox-instance auto: {err}"))?;

    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !root.is_empty() {
            return Ok(PathBuf::from(root));
        }
    }

    std::env::current_dir().map_err(|err| anyhow!("Failed to resolve current directory: {err}"))
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

/// Compute a deterministic 64-bit FNV-1a hash for stable instance-id derivation.
///
/// This hash is used only for resource naming (not cryptographic security).
/// We prefer FNV-1a here because it is tiny, fast, stable across platforms,
/// and available without adding extra dependencies.
fn compute_stable_fnv1a_64_hash(input: &[u8]) -> u64 {
    const FNV1A_64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV1A_64_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV1A_64_OFFSET_BASIS;
    for byte in input {
        // FNV-1a step order is XOR first, then multiply.
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV1A_64_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_manual_name_rules() {
        assert!(is_valid_instance_id("abc"));
        assert!(is_valid_instance_id("abc-123"));
        assert!(!is_valid_instance_id(""));
        assert!(!is_valid_instance_id("-abc"));
        assert!(!is_valid_instance_id("abc_def"));
        assert!(!is_valid_instance_id("Abc"));
        assert!(!is_valid_instance_id(&"a".repeat(33)));
    }

    #[test]
    fn auto_derivation_is_stable_for_same_path() {
        let p = "/tmp/worktree-a";
        let a = format!("wt-{:012x}", compute_stable_fnv1a_64_hash(p.as_bytes()));
        let b = format!("wt-{:012x}", compute_stable_fnv1a_64_hash(p.as_bytes()));
        assert_eq!(a, b);
    }

    #[test]
    fn auto_derivation_differs_for_different_paths() {
        let a = format!(
            "wt-{:012x}",
            compute_stable_fnv1a_64_hash("/tmp/worktree-a".as_bytes())
        );
        let b = format!(
            "wt-{:012x}",
            compute_stable_fnv1a_64_hash("/tmp/worktree-b".as_bytes())
        );
        assert_ne!(a, b);
    }

    #[test]
    fn invalid_manual_name_is_rejected() {
        let err = resolve_sandbox_profile(Some("bad_name")).expect_err("expected validation error");
        assert!(err.to_string().contains("Expected [a-z0-9][a-z0-9-]{0,31}"));
    }
}
