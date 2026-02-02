//! Rendered views of configuration data for interactive output.

use opencode_cloud_core::Config;
use serde::Serialize;
use serde::ser::SerializeStruct;

const REDACTED_VALUE: &str = "REDACTED";
const REDACTED_PASSWORD: &str = "********";

#[derive(Debug)]
pub struct RedactedConfig<'a> {
    config: &'a Config,
}

impl<'a> RedactedConfig<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }
}

impl Serialize for RedactedConfig<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let config = self.config;
        assert_all_fields_covered(config);

        let mut state = serializer.serialize_struct("Config", 19)?;
        state.serialize_field("version", &config.version)?;
        state.serialize_field("opencode_web_port", &config.opencode_web_port)?;
        state.serialize_field("bind", &config.bind)?;
        state.serialize_field("auto_restart", &config.auto_restart)?;
        state.serialize_field("boot_mode", &config.boot_mode)?;
        state.serialize_field("restart_retries", &config.restart_retries)?;
        state.serialize_field("restart_delay", &config.restart_delay)?;
        state.serialize_field("auth_username", &config.auth_username)?;

        let redacted_password = config
            .auth_password
            .as_ref()
            .map(|_| REDACTED_PASSWORD.to_string());
        state.serialize_field("auth_password", &redacted_password)?;

        let redacted_env = redact_env_entries(&config.container_env);
        state.serialize_field("container_env", &redacted_env)?;

        state.serialize_field("bind_address", &config.bind_address)?;
        state.serialize_field("trust_proxy", &config.trust_proxy)?;
        state.serialize_field(
            "allow_unauthenticated_network",
            &config.allow_unauthenticated_network,
        )?;
        state.serialize_field("rate_limit_attempts", &config.rate_limit_attempts)?;
        state.serialize_field(
            "rate_limit_window_seconds",
            &config.rate_limit_window_seconds,
        )?;
        state.serialize_field("users", &config.users)?;
        state.serialize_field("image_source", &config.image_source)?;
        state.serialize_field("update_check", &config.update_check)?;
        state.serialize_field("mounts", &config.mounts)?;
        state.end()
    }
}

pub fn render_config_snapshot(config: &Config) -> String {
    serde_json::to_string_pretty(&RedactedConfig::new(config))
        .unwrap_or_else(|_| "<failed to format config>".to_string())
}

fn redact_env_entries(entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| redact_env_entry(entry))
        .collect()
}

fn redact_env_entry(entry: &str) -> String {
    if let Some((key, _)) = entry.split_once('=') {
        format!("{key}={REDACTED_VALUE}")
    } else {
        REDACTED_VALUE.to_string()
    }
}

fn assert_all_fields_covered(config: &Config) {
    let Config {
        version: _,
        opencode_web_port: _,
        bind: _,
        auto_restart: _,
        boot_mode: _,
        restart_retries: _,
        restart_delay: _,
        auth_username: _,
        auth_password: _,
        container_env: _,
        bind_address: _,
        trust_proxy: _,
        allow_unauthenticated_network: _,
        rate_limit_attempts: _,
        rate_limit_window_seconds: _,
        users: _,
        cockpit_port: _,
        cockpit_enabled: _,
        image_source: _,
        update_check: _,
        mounts: _,
    } = config;
}
