use std::path::PathBuf;

use anyhow::{bail, Result};
use async_trait::async_trait;
use oauth2::{AuthUrl, RedirectUrl, Scope, TokenUrl};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use url::Url;

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
pub struct ServicePresetConfig {
    pub auth_url: AuthUrl,
    pub token_url: Option<TokenUrl>,
    pub base_url: Url,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
pub struct ClientRawConfig {
    pub preset_name: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_url: RedirectUrl,
    pub scopes: Vec<Scope>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
pub struct ClientConfig {
    pub preset: ServicePresetConfig,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_url: RedirectUrl,
    pub scopes: Vec<Scope>,
}

impl ClientConfig {
    pub fn new(
        preset: ServicePresetConfig,
        ClientRawConfig {
            client_id,
            client_secret,
            redirect_url,
            scopes,
            ..
        }: ClientRawConfig,
    ) -> Self {
        Self {
            preset,
            client_id,
            client_secret,
            redirect_url,
            scopes,
        }
    }
}

#[async_trait]
pub trait ConfigOpener {
    async fn config_dir(&self) -> Result<PathBuf>;
    async fn open_preset_config(&self, name: &str) -> Result<ServicePresetConfig> {
        let config_dir = self.config_dir().await?;
        let path = config_dir.join(format!("{name}.preset.toml"));
        let text = tokio::fs::read_to_string(&path).await?;
        let config = toml::from_str(&text)?;
        Ok(config)
    }
    async fn open_client_raw_config(&self, name: &str) -> Result<ClientRawConfig> {
        let config_dir = self.config_dir().await?;
        let path = config_dir.join(format!("{name}.client.toml"));
        log::info!("{path:?}");
        let text = tokio::fs::read_to_string(&path).await?;
        let config = toml::from_str(&text)?;
        Ok(config)
    }
    async fn open_client_config(&self, name: &str) -> Result<ClientConfig> {
        let raw = self.open_client_raw_config(name).await?;
        let preset = self.open_preset_config(&raw.preset_name).await?;
        Ok(ClientConfig::new(preset, raw))
    }
}

#[async_trait]
impl ConfigOpener for AppHandle {
    async fn config_dir(&self) -> Result<PathBuf> {
        let Some(config_dir) = self.path_resolver().app_config_dir() else {
            bail!("Failed to get app_config_dir.");
        };
        let config_dir = config_dir.join("config");
        if !config_dir.exists() {
            tokio::fs::create_dir_all(&config_dir).await?;
        }
        Ok(config_dir)
    }
}
