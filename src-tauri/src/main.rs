// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use oauth2::{basic::BasicTokenType, EmptyExtraTokenFields, StandardTokenResponse};
use tauri::{AppHandle, Manager};
use tauri_plugin_log::LogTarget;
use tokio::sync::Mutex;
use url::Url;

use crate::{
    auth::OAuth2Authorizer,
    config::{ClientConfig, ConfigOpener},
};

#[derive(Default, Debug)]
pub struct AuthorizerState(Mutex<HashMap<String, OAuth2Authorizer>>);

impl Deref for AuthorizerState {
    type Target = Mutex<HashMap<String, OAuth2Authorizer>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AuthorizerState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[tauri::command(rename_all = "camelCase")]
async fn get_client_config(app: AppHandle, name: &str) -> Result<ClientConfig, String> {
    let config = app
        .open_client_config(name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command(rename_all = "camelCase")]
async fn get_authorize_url(app: AppHandle, name: &str) -> Result<Url, String> {
    let config = app
        .open_client_config(name)
        .await
        .map_err(|e| e.to_string())?;
    let authorizer = OAuth2Authorizer::new(config);
    let authorize_url = authorizer.authorize_url().clone();
    let authorizers = app.state::<AuthorizerState>();
    authorizers
        .lock()
        .await
        .insert(name.to_string(), authorizer);
    Ok(authorize_url)
}

#[tauri::command(rename_all = "camelCase")]
async fn exchange_redirect_url(
    app: AppHandle,
    name: &str,
    redirect_url: Url,
) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, String> {
    let authorizers = app.state::<AuthorizerState>();
    let authorizer = authorizers.lock().await.remove(name);
    if let Some(authorizer) = authorizer {
        authorizer
            .try_into_token_with_redirect_url(redirect_url)
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("Not found name".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AuthorizerState::default())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_client_config,
            get_authorize_url,
            exchange_redirect_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
