//! Canonical embedded Docker runtime assets shared across crates.
//!
//! Keep these constants as the single source of truth for runtime drift checks
//! and Docker build-context packaging.

pub const OPENCODE_CLOUD_BOOTSTRAP_SH: &[u8] = include_bytes!("files/opencode-cloud-bootstrap.sh");
pub const ENTRYPOINT_SH: &[u8] = include_bytes!("files/entrypoint.sh");
pub const HEALTHCHECK_SH: &[u8] = include_bytes!("files/healthcheck.sh");
