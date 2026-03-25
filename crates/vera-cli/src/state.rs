//! Persistent CLI state for agent-friendly setup and installs.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_api: Option<ApiEndpointConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranker_api: Option<ApiEndpointConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_config: Option<vera_core::config::VeraConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiEndpointConfig {
    pub base_url: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredSecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranker_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiSetupInput {
    pub base_url: String,
    pub model_id: String,
    pub api_key: String,
}

pub fn load_saved_config() -> Result<StoredConfig> {
    load_json_file(&config_path()?)
}

pub fn load_saved_secrets() -> Result<StoredSecrets> {
    load_json_file(&credentials_path()?)
}

pub fn save_local_mode(enabled: bool) -> Result<()> {
    let mut config = load_saved_config()?;
    config.local_mode = Some(enabled);
    save_config(&config)
}

pub fn save_runtime_config(config: &vera_core::config::VeraConfig) -> Result<()> {
    let mut stored = load_saved_config()?;
    stored.core_config = Some(config.clone());
    save_config(&stored)
}

pub fn save_api_setup(embedding: &ApiSetupInput, reranker: Option<&ApiSetupInput>) -> Result<()> {
    let mut config = load_saved_config()?;
    config.local_mode = Some(false);
    config.embedding_api = Some(ApiEndpointConfig {
        base_url: embedding.base_url.clone(),
        model_id: embedding.model_id.clone(),
    });
    config.reranker_api = reranker.map(|cfg| ApiEndpointConfig {
        base_url: cfg.base_url.clone(),
        model_id: cfg.model_id.clone(),
    });
    save_config(&config)?;

    let mut secrets = load_saved_secrets()?;
    secrets.embedding_api_key = Some(embedding.api_key.clone());
    secrets.reranker_api_key = reranker.map(|cfg| cfg.api_key.clone());
    save_secrets(&secrets)
}

pub fn load_runtime_config() -> Result<vera_core::config::VeraConfig> {
    let default = vera_core::config::VeraConfig::default();
    Ok(load_saved_config()?.core_config.unwrap_or(default))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(vera_dir()?.join("config.json"))
}

pub fn credentials_path() -> Result<PathBuf> {
    Ok(vera_dir()?.join("credentials.json"))
}

pub fn vera_dir() -> Result<PathBuf> {
    vera_core::local_models::vera_home_dir()
}

pub fn user_home_dir() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("VERA_USER_HOME") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    dirs::home_dir().context("Could not find home directory")
}

pub fn apply_saved_env() -> Result<()> {
    apply_saved_env_impl(false)
}

pub fn apply_saved_env_force() -> Result<()> {
    apply_saved_env_impl(true)
}

fn apply_saved_env_impl(force: bool) -> Result<()> {
    let config = load_saved_config()?;
    let secrets = load_saved_secrets()?;

    if let Some(local_mode) = config.local_mode {
        set_env_value("VERA_LOCAL", if local_mode { "1" } else { "0" }, force);
    }

    if let Some(embedding) = config.embedding_api.as_ref() {
        set_env_value("EMBEDDING_MODEL_BASE_URL", &embedding.base_url, force);
        set_env_value("EMBEDDING_MODEL_ID", &embedding.model_id, force);
    }
    if let Some(api_key) = secrets.embedding_api_key.as_deref() {
        set_env_value("EMBEDDING_MODEL_API_KEY", api_key, force);
    }

    if let Some(reranker) = config.reranker_api.as_ref() {
        set_env_value("RERANKER_MODEL_BASE_URL", &reranker.base_url, force);
        set_env_value("RERANKER_MODEL_ID", &reranker.model_id, force);
    }
    if let Some(api_key) = secrets.reranker_api_key.as_deref() {
        set_env_value("RERANKER_MODEL_API_KEY", api_key, force);
    }

    Ok(())
}

fn save_config(config: &StoredConfig) -> Result<()> {
    write_json_file(&config_path()?, config)
}

fn save_secrets(secrets: &StoredSecrets) -> Result<()> {
    write_json_file(&credentials_path()?, secrets)
}

fn load_json_file<T>(path: &Path) -> Result<T>
where
    T: Default + for<'de> Deserialize<'de>,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let contents = fs::read(path)
        .with_context(|| format!("failed to read persistent state: {}", path.display()))?;
    if contents.is_empty() {
        return Ok(T::default());
    }

    serde_json::from_slice(&contents)
        .with_context(|| format!("failed to parse persistent state: {}", path.display()))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let contents = serde_json::to_vec_pretty(value)
        .with_context(|| format!("failed to serialize state for {}", path.display()))?;
    write_private_file(path, &contents)
}

fn write_private_file(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(&tmp_path)
        .with_context(|| format!("failed to open {}", tmp_path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", tmp_path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to finalize {}", tmp_path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", tmp_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to set permissions on {}", tmp_path.display()))?;
    }

    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("failed to replace existing {}", path.display()))?;
    }
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to move {} into place as {}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn set_env_value(key: &str, value: &str, force: bool) {
    if force || std::env::var_os(key).is_none() {
        set_process_env(key, value);
    }
}

fn set_process_env(key: &str, value: &str) {
    // Safe because Vera only mutates process environment during single-threaded
    // CLI startup, before any background work or runtime threads are created.
    unsafe {
        std::env::set_var(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_config_defaults_are_empty() {
        let config = StoredConfig::default();
        assert!(config.local_mode.is_none());
        assert!(config.embedding_api.is_none());
        assert!(config.reranker_api.is_none());
        assert!(config.core_config.is_none());
    }

    #[test]
    fn stored_secrets_default_empty() {
        let secrets = StoredSecrets::default();
        assert!(secrets.embedding_api_key.is_none());
        assert!(secrets.reranker_api_key.is_none());
    }
}
