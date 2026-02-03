//! Docker registry API helpers.
//!
//! Provides lightweight helpers for querying registry manifests and configs
//! without pulling images.

use super::DockerError;
use reqwest::header::ACCEPT;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct TokenResponse {
    token: Option<String>,
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct ManifestConfig {
    digest: String,
}

#[derive(Deserialize)]
struct Manifest {
    config: ManifestConfig,
}

#[derive(Deserialize)]
struct ManifestList {
    manifests: Vec<ManifestDescriptor>,
}

#[derive(Deserialize)]
struct ManifestDescriptor {
    digest: String,
    platform: Option<ManifestPlatform>,
}

#[derive(Deserialize)]
struct ManifestPlatform {
    architecture: Option<String>,
    os: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ManifestResponse {
    Single(Manifest),
    List(ManifestList),
}

#[derive(Deserialize)]
struct ImageConfig {
    config: Option<ImageConfigDetails>,
}

#[derive(Deserialize)]
struct ImageConfigDetails {
    #[serde(rename = "Labels")]
    labels: Option<HashMap<String, String>>,
}

pub async fn fetch_registry_version(
    registry_base: &str,
    token_url: &str,
    repo: &str,
    tag: &str,
    maybe_manifest_digest: Option<&str>,
    label_key: &str,
) -> Result<Option<String>, DockerError> {
    let client = reqwest::Client::new();
    let token = fetch_registry_token(&client, token_url).await?;
    let manifest = if let Some(digest) = maybe_manifest_digest {
        match fetch_registry_manifest(&client, registry_base, repo, digest, &token).await {
            Ok(manifest) => manifest,
            Err(digest_err) => fetch_registry_manifest(&client, registry_base, repo, tag, &token)
                .await
                .map_err(|tag_err| {
                    DockerError::Connection(format!(
                        "Failed to fetch registry manifest. Digest error: {digest_err}. Tag error: {tag_err}"
                    ))
                })?,
        }
    } else {
        fetch_registry_manifest(&client, registry_base, repo, tag, &token).await?
    };
    let image_config = fetch_registry_image_config(
        &client,
        registry_base,
        repo,
        &manifest.config.digest,
        &token,
    )
    .await?;

    let maybe_version = image_config
        .config
        .and_then(|details| details.labels)
        .and_then(|labels| labels.get(label_key).cloned());

    Ok(maybe_version)
}

async fn fetch_registry_token(
    client: &reqwest::Client,
    token_url: &str,
) -> Result<String, DockerError> {
    let response = client
        .get(token_url)
        .send()
        .await
        .map_err(|e| DockerError::Connection(format!("Failed to fetch registry token: {e}")))?;

    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| DockerError::Connection(format!("Failed to decode registry token: {e}")))?;
    let token = token_response
        .token
        .or(token_response.access_token)
        .ok_or_else(|| DockerError::Connection("Registry token missing".to_string()))?;

    Ok(token)
}

async fn fetch_registry_manifest(
    client: &reqwest::Client,
    registry_base: &str,
    repo: &str,
    reference: &str,
    token: &str,
) -> Result<Manifest, DockerError> {
    let mut current_reference = reference.to_string();
    loop {
        let manifest_url = format!("{registry_base}/v2/{repo}/manifests/{current_reference}");
        let response = client
            .get(&manifest_url)
            .header(
                ACCEPT,
                "application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json",
            )
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| {
                DockerError::Connection(format!("Failed to fetch registry manifest: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(DockerError::Connection(format!(
                "Manifest not found for {repo}:{current_reference}"
            )));
        }

        let manifest_response: ManifestResponse = response
            .json()
            .await
            .map_err(|e| DockerError::Connection(format!("Failed to decode manifest: {e}")))?;

        match manifest_response {
            ManifestResponse::Single(manifest) => return Ok(manifest),
            ManifestResponse::List(list) => {
                let digest = select_manifest_digest(&list).ok_or_else(|| {
                    DockerError::Connection(format!(
                        "No manifests available for {repo}:{current_reference}"
                    ))
                })?;
                current_reference = digest;
            }
        }
    }
}

fn select_manifest_digest(list: &ManifestList) -> Option<String> {
    let preferred = list.manifests.iter().find(|manifest| {
        let Some(platform) = &manifest.platform else {
            return false;
        };
        platform.os.as_deref() == Some("linux") && platform.architecture.as_deref() == Some("amd64")
    });

    preferred
        .or_else(|| list.manifests.first())
        .map(|manifest| manifest.digest.clone())
}

async fn fetch_registry_image_config(
    client: &reqwest::Client,
    registry_base: &str,
    repo: &str,
    digest: &str,
    token: &str,
) -> Result<ImageConfig, DockerError> {
    let config_url = format!("{registry_base}/v2/{repo}/blobs/{digest}");
    let response = client
        .get(&config_url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| DockerError::Connection(format!("Failed to fetch image config: {e}")))?;

    if !response.status().is_success() {
        return Err(DockerError::Connection(format!(
            "Failed to fetch image config: HTTP {}",
            response.status()
        )));
    }

    let config = response
        .json()
        .await
        .map_err(|e| DockerError::Connection(format!("Failed to decode image config: {e}")))?;

    Ok(config)
}
