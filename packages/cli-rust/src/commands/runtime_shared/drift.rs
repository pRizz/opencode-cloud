//! Local-vs-container runtime asset drift detection.
//!
//! This module compares embedded local runtime assets against the currently
//! running container copies to detect stale local-dev/container mismatches.

use opencode_cloud_core::docker::{
    CONTAINER_NAME, DockerClient, container_is_running, exec_command_with_status,
};

pub const REBUILD_CACHED_COMMAND: &str = "occ start --cached-rebuild-sandbox-image";
pub const REBUILD_FULL_COMMAND: &str = "occ start --full-rebuild-sandbox-image";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeAssetDrift {
    pub drift_detected: bool,
    pub mismatched_assets: Vec<String>,
    pub diagnostics: Vec<String>,
}

impl RuntimeAssetDrift {
    fn from_outcomes(mismatched_assets: Vec<String>, diagnostics: Vec<String>) -> Self {
        Self {
            drift_detected: !mismatched_assets.is_empty(),
            mismatched_assets,
            diagnostics,
        }
    }
}

struct RuntimeAsset {
    name: &'static str,
    container_path: &'static str,
    expected_bytes: &'static [u8],
}

const TRACKED_RUNTIME_ASSETS: &[RuntimeAsset] = &[
    RuntimeAsset {
        name: "bootstrap helper",
        container_path: "/usr/local/bin/opencode-cloud-bootstrap",
        expected_bytes: include_bytes!(
            "../../../../core/src/docker/files/opencode-cloud-bootstrap.sh"
        ),
    },
    RuntimeAsset {
        name: "entrypoint",
        container_path: "/usr/local/bin/entrypoint.sh",
        expected_bytes: include_bytes!("../../../../core/src/docker/files/entrypoint.sh"),
    },
    RuntimeAsset {
        name: "healthcheck",
        container_path: "/usr/local/bin/healthcheck.sh",
        expected_bytes: include_bytes!("../../../../core/src/docker/files/healthcheck.sh"),
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum AssetProbeOutcome {
    Match,
    Mismatch,
    ProbeFailed(String),
}

pub async fn detect_runtime_asset_drift(client: &DockerClient) -> RuntimeAssetDrift {
    let running = match container_is_running(client, CONTAINER_NAME).await {
        Ok(running) => running,
        Err(err) => {
            return RuntimeAssetDrift::from_outcomes(
                Vec::new(),
                vec![format!("failed to check running container state: {err}")],
            );
        }
    };

    if !running {
        return RuntimeAssetDrift::default();
    }

    let mut results = Vec::with_capacity(TRACKED_RUNTIME_ASSETS.len());
    for asset in TRACKED_RUNTIME_ASSETS {
        let outcome = probe_asset(client, asset).await;
        results.push((asset.name.to_string(), outcome));
    }
    build_drift_report(&results)
}

pub fn stale_container_warning_lines(report: &RuntimeAssetDrift) -> Vec<String> {
    if !report.drift_detected {
        return Vec::new();
    }

    let mismatched = report.mismatched_assets.join(", ");
    vec![
        "Running container is out of sync with local development assets.".to_string(),
        format!("Mismatched assets: {mismatched}"),
        format!("Rebuild with: {REBUILD_CACHED_COMMAND}"),
        format!("If needed (no cache): {REBUILD_FULL_COMMAND}"),
    ]
}

async fn probe_asset(client: &DockerClient, asset: &RuntimeAsset) -> AssetProbeOutcome {
    let command = vec!["cat", asset.container_path];
    let (output, status) = match exec_command_with_status(client, CONTAINER_NAME, command).await {
        Ok(result) => result,
        Err(err) => {
            return AssetProbeOutcome::ProbeFailed(format!("exec failed: {err}"));
        }
    };

    if status != 0 {
        return AssetProbeOutcome::ProbeFailed(format!("exit status {status}"));
    }

    if output.as_bytes() == asset.expected_bytes {
        AssetProbeOutcome::Match
    } else {
        AssetProbeOutcome::Mismatch
    }
}

fn build_drift_report(results: &[(String, AssetProbeOutcome)]) -> RuntimeAssetDrift {
    let mut mismatched_assets = Vec::new();
    let mut diagnostics = Vec::new();

    for (name, outcome) in results {
        match outcome {
            AssetProbeOutcome::Match => {}
            AssetProbeOutcome::Mismatch => mismatched_assets.push(name.clone()),
            AssetProbeOutcome::ProbeFailed(detail) => {
                diagnostics.push(format!("{name}: {detail}"));
            }
        }
    }

    RuntimeAssetDrift::from_outcomes(mismatched_assets, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatch_detection_marks_drift() {
        let report =
            build_drift_report(&[("bootstrap helper".to_string(), AssetProbeOutcome::Mismatch)]);
        assert!(report.drift_detected);
        assert_eq!(
            report.mismatched_assets,
            vec!["bootstrap helper".to_string()]
        );
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn no_drift_when_all_assets_match() {
        let report = build_drift_report(&[
            ("bootstrap helper".to_string(), AssetProbeOutcome::Match),
            ("entrypoint".to_string(), AssetProbeOutcome::Match),
            ("healthcheck".to_string(), AssetProbeOutcome::Match),
        ]);
        assert!(!report.drift_detected);
        assert!(report.mismatched_assets.is_empty());
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn partial_drift_reports_only_mismatched_assets() {
        let report = build_drift_report(&[
            ("bootstrap helper".to_string(), AssetProbeOutcome::Mismatch),
            ("entrypoint".to_string(), AssetProbeOutcome::Match),
            ("healthcheck".to_string(), AssetProbeOutcome::Mismatch),
        ]);
        assert!(report.drift_detected);
        assert_eq!(
            report.mismatched_assets,
            vec!["bootstrap helper".to_string(), "healthcheck".to_string()]
        );
    }

    #[test]
    fn warning_lines_include_rebuild_recommendations() {
        let report = RuntimeAssetDrift::from_outcomes(
            vec!["bootstrap helper".to_string(), "entrypoint".to_string()],
            Vec::new(),
        );
        let lines = stale_container_warning_lines(&report);
        assert!(lines.iter().any(|line| line.contains("bootstrap helper")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains(REBUILD_CACHED_COMMAND))
        );
        assert!(lines.iter().any(|line| line.contains(REBUILD_FULL_COMMAND)));
    }
}
