use opencode_cloud_core::docker::{CONTAINER_NAME, DockerClient, exec_command_with_status};
use serde_json::Value;

const BOOTSTRAP_HELPER_PATH: &str = "/usr/local/bin/opencode-cloud-bootstrap";
pub(crate) const IOTP_FALLBACK_COMMAND: &str = "occ logs | grep -F \"INITIAL ONE-TIME PASSWORD (IOTP): \" | tail -n1 | sed 's/.*INITIAL ONE-TIME PASSWORD (IOTP): //'";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IotpState {
    ActiveUnused,
    InactiveUsersConfigured,
    InactiveCompleted,
    InactiveNotInitialized,
    ErrorInvalidState,
    ErrorHelper,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IotpSnapshot {
    pub(crate) state: IotpState,
    pub(crate) state_label: String,
    pub(crate) otp: Option<String>,
    pub(crate) detail: Option<String>,
}

impl IotpSnapshot {
    pub(crate) fn unavailable(detail: impl Into<String>) -> Self {
        Self {
            state: IotpState::Unavailable,
            state_label: "unavailable".to_string(),
            otp: None,
            detail: Some(detail.into()),
        }
    }
}

pub(crate) async fn fetch_iotp_snapshot(client: &DockerClient) -> IotpSnapshot {
    let (output, status) = match exec_command_with_status(
        client,
        CONTAINER_NAME,
        vec![BOOTSTRAP_HELPER_PATH, "status", "--include-secret"],
    )
    .await
    {
        Ok(result) => result,
        Err(err) => {
            return IotpSnapshot::unavailable(format!("failed to query bootstrap helper: {err}"));
        }
    };

    if status != 0 {
        let detail = if output.trim().is_empty() {
            format!("bootstrap helper exited with status {status}")
        } else {
            format!(
                "bootstrap helper exited with status {status}: {}",
                output.trim()
            )
        };
        return IotpSnapshot::unavailable(detail);
    }

    match parse_snapshot_output(&output) {
        Ok(snapshot) => snapshot,
        Err(err) => IotpSnapshot::unavailable(err),
    }
}

fn snapshot(
    state: IotpState,
    state_label: &str,
    otp: Option<String>,
    detail: Option<String>,
) -> IotpSnapshot {
    IotpSnapshot {
        state,
        state_label: state_label.to_string(),
        otp,
        detail,
    }
}

fn parse_snapshot_output(output: &str) -> Result<IotpSnapshot, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("bootstrap helper returned empty output".to_string());
    }

    let payload = parse_json_payload(trimmed)?;
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    if !ok {
        let code = payload
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let message = payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown helper error");
        return Ok(snapshot(
            IotpState::ErrorHelper,
            "error",
            None,
            Some(format!("{code}: {message}")),
        ));
    }

    let active = payload
        .get("active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if active {
        let otp = payload
            .get("otp")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        if let Some(token) = otp {
            return Ok(snapshot(
                IotpState::ActiveUnused,
                "unused (active)",
                Some(token),
                None,
            ));
        }
        return Ok(snapshot(
            IotpState::ErrorInvalidState,
            "error (invalid bootstrap state)",
            None,
            Some("bootstrap helper reported active state without an OTP value".to_string()),
        ));
    }

    let reason = payload
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mapped = match reason {
        "user_exists" => snapshot(
            IotpState::InactiveUsersConfigured,
            "inactive (users configured)",
            None,
            None,
        ),
        "completed" => snapshot(IotpState::InactiveCompleted, "used (completed)", None, None),
        "not_initialized" => snapshot(
            IotpState::InactiveNotInitialized,
            "inactive (not initialized)",
            None,
            None,
        ),
        "invalid_state" | "invalid_secret" => snapshot(
            IotpState::ErrorInvalidState,
            "error (invalid bootstrap state)",
            None,
            Some(format!("bootstrap helper reported reason: {reason}")),
        ),
        other => snapshot(
            IotpState::Unavailable,
            "unavailable",
            None,
            Some(format!("bootstrap helper reported reason: {other}")),
        ),
    };
    Ok(mapped)
}

fn parse_json_payload(raw: &str) -> Result<Value, String> {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Ok(value);
    }

    for line in raw.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            return Ok(value);
        }
    }

    Err("failed to parse bootstrap helper JSON output".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_active_snapshot_with_otp() {
        let raw = r#"{"ok":true,"active":true,"created_at":"2026-02-08T00:00:00Z","otp":"abc123"}"#;
        let snapshot = parse_snapshot_output(raw).expect("parse should succeed");
        assert_eq!(snapshot.state, IotpState::ActiveUnused);
        assert_eq!(snapshot.state_label, "unused (active)");
        assert_eq!(snapshot.otp, Some("abc123".to_string()));
        assert!(snapshot.detail.is_none());
    }

    #[test]
    fn parse_user_exists_reason() {
        let raw = r#"{"ok":true,"active":false,"reason":"user_exists"}"#;
        let snapshot = parse_snapshot_output(raw).expect("parse should succeed");
        assert_eq!(snapshot.state, IotpState::InactiveUsersConfigured);
        assert_eq!(snapshot.state_label, "inactive (users configured)");
        assert!(snapshot.otp.is_none());
    }

    #[test]
    fn parse_completed_reason() {
        let raw = r#"{"ok":true,"active":false,"reason":"completed"}"#;
        let snapshot = parse_snapshot_output(raw).expect("parse should succeed");
        assert_eq!(snapshot.state, IotpState::InactiveCompleted);
        assert_eq!(snapshot.state_label, "used (completed)");
    }

    #[test]
    fn parse_not_initialized_reason() {
        let raw = r#"{"ok":true,"active":false,"reason":"not_initialized"}"#;
        let snapshot = parse_snapshot_output(raw).expect("parse should succeed");
        assert_eq!(snapshot.state, IotpState::InactiveNotInitialized);
        assert_eq!(snapshot.state_label, "inactive (not initialized)");
    }

    #[test]
    fn parse_invalid_state_reason() {
        let raw = r#"{"ok":true,"active":false,"reason":"invalid_secret"}"#;
        let snapshot = parse_snapshot_output(raw).expect("parse should succeed");
        assert_eq!(snapshot.state, IotpState::ErrorInvalidState);
        assert_eq!(snapshot.state_label, "error (invalid bootstrap state)");
        assert!(
            snapshot
                .detail
                .expect("detail should be present")
                .contains("invalid_secret")
        );
    }

    #[test]
    fn parse_helper_error_payload() {
        let raw = r#"{"ok":false,"code":"otp_invalid","message":"bad otp","status":401}"#;
        let snapshot = parse_snapshot_output(raw).expect("parse should succeed");
        assert_eq!(snapshot.state, IotpState::ErrorHelper);
        assert_eq!(snapshot.state_label, "error");
        assert_eq!(snapshot.detail, Some("otp_invalid: bad otp".to_string()));
    }

    #[test]
    fn malformed_json_is_error() {
        let err = parse_snapshot_output("not-json").expect_err("parse should fail");
        assert!(err.contains("failed to parse"));
    }
}
