//! Disk usage reporting utilities.

use anyhow::{Result, anyhow};
use opencode_cloud_core::bollard::models::SystemDataUsageResponse;
use opencode_cloud_core::docker::DockerClient;
use sysinfo::Disks;

#[derive(Clone, Copy)]
pub struct DiskUsageReport {
    pub images: Option<i64>,
    pub containers: Option<i64>,
    pub volumes: Option<i64>,
    pub build_cache: Option<i64>,
    pub total: Option<i64>,
}

#[derive(Clone, Copy)]
pub struct HostDiskReport {
    pub total: u64,
    pub available: u64,
    pub used: u64,
}

pub fn format_disk_usage_report(
    stage: &str,
    report: DiskUsageReport,
    baseline: Option<DiskUsageReport>,
) -> Vec<String> {
    let mut lines = vec![
        format!("Docker disk usage ({stage}):"),
        format!("  Images:      {}", format_usage_value(report.images)),
        format!("  Containers:  {}", format_usage_value(report.containers)),
        format!("  Volumes:     {}", format_usage_value(report.volumes)),
        format!("  Build cache: {}", format_usage_value(report.build_cache)),
        format!("  Total:       {}", format_usage_value(report.total)),
    ];
    if let Some(delta) = format_delta_i64(report.total, baseline.and_then(|b| b.total)) {
        lines.push(format!("  Delta:       {delta}"));
    }
    lines
}

pub fn format_host_disk_report(
    stage: &str,
    report: HostDiskReport,
    baseline: Option<HostDiskReport>,
) -> Vec<String> {
    let mut lines = vec![
        format!("Host disk ({stage}):"),
        format!(
            "  Total:       {}",
            format_usage_value_u64(Some(report.total))
        ),
        format!(
            "  Available:   {}",
            format_usage_value_u64(Some(report.available))
        ),
        format!(
            "  Used:        {}",
            format_usage_value_u64(Some(report.used))
        ),
    ];
    if let Some(delta) = format_delta_u64(Some(report.used), baseline.map(|baseline| baseline.used))
    {
        lines.push(format!("  Delta used:  {delta}"));
    }
    lines
}

pub async fn get_disk_usage_report(client: &DockerClient) -> Result<DiskUsageReport> {
    use opencode_cloud_core::bollard::query_parameters::DataUsageOptions;
    let data_usage = client
        .inner()
        .df(None::<DataUsageOptions>)
        .await
        .map_err(|e| anyhow!("Failed to read Docker disk usage: {e}"))?;
    Ok(build_disk_usage_report(&data_usage))
}

pub fn get_host_disk_report(client: &DockerClient) -> Result<Option<HostDiskReport>> {
    if client.is_remote() {
        return Ok(None);
    }
    let disks = Disks::new_with_refreshed_list();
    Ok(build_host_disk_report(&disks))
}

pub fn format_bytes_i64(value: i64) -> String {
    if value < 0 {
        return "unknown".to_string();
    }
    format_bytes_u64(value as u64)
}

fn build_disk_usage_report(data_usage: &SystemDataUsageResponse) -> DiskUsageReport {
    // Bollard v0.20+ uses new disk usage structures
    let images = data_usage
        .images_disk_usage
        .as_ref()
        .and_then(|u| u.total_size);
    let containers = data_usage
        .containers_disk_usage
        .as_ref()
        .and_then(|u| u.total_size);
    let volumes = data_usage
        .volumes_disk_usage
        .as_ref()
        .and_then(|u| u.total_size);
    let build_cache = data_usage
        .build_cache_disk_usage
        .as_ref()
        .and_then(|u| u.total_size);

    let total = match (images, containers, volumes, build_cache) {
        (Some(images), Some(containers), Some(volumes), Some(build_cache)) => {
            Some(images + containers + volumes + build_cache)
        }
        _ => None,
    };

    DiskUsageReport {
        images,
        containers,
        volumes,
        build_cache,
        total,
    }
}

fn build_host_disk_report(disks: &Disks) -> Option<HostDiskReport> {
    if disks.list().is_empty() {
        return None;
    }
    let mut total = 0u64;
    let mut available = 0u64;
    for disk in disks.list() {
        total = total.saturating_add(disk.total_space());
        available = available.saturating_add(disk.available_space());
    }
    let used = total.saturating_sub(available);
    Some(HostDiskReport {
        total,
        available,
        used,
    })
}

fn format_delta_i64(after: Option<i64>, before: Option<i64>) -> Option<String> {
    let (after, before) = (after?, before?);
    let delta = after - before;
    let sign = if delta >= 0 { "+" } else { "-" };
    Some(format!("{sign}{}", format_bytes_u64(delta.unsigned_abs())))
}

fn format_delta_u64(after: Option<u64>, before: Option<u64>) -> Option<String> {
    let (after, before) = (after?, before?);
    if after >= before {
        Some(format!("+{}", format_bytes_u64(after - before)))
    } else {
        Some(format!("-{}", format_bytes_u64(before - after)))
    }
}

fn format_bytes_u64(value: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = value as f64;
    let mut index = 0usize;
    while size >= 1024.0 && index + 1 < units.len() {
        size /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{value} {}", units[index])
    } else {
        format!("{size:.2} {}", units[index])
    }
}

fn format_usage_value(value: Option<i64>) -> String {
    value
        .map(format_bytes_i64)
        .unwrap_or_else(|| "unknown".to_string())
}

fn format_usage_value_u64(value: Option<u64>) -> String {
    value
        .map(format_bytes_u64)
        .unwrap_or_else(|| "unknown".to_string())
}
