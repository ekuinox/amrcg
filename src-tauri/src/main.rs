// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use anyhow::{bail, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use log::LevelFilter;
use oauth2::TokenResponse;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_log::LogTarget;
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
use url::Url;

use crate::{
    auth::OAuth2Authorizer,
    config::{ClientConfig, ConfigOpener},
};

fn main() {
    tauri::Builder::default()
        .manage(AuthorizerThreadsState::default())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(LevelFilter::Warn)
                .level_for(env!("CARGO_PKG_NAME"), LevelFilter::Info)
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_client_config,
            start_server,
            stop_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// `{name}.client.toml` を取得して返す
#[tauri::command(rename_all = "camelCase")]
async fn get_client_config(app: AppHandle, name: &str) -> Result<ClientConfig, String> {
    let config = app
        .open_client_config(name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(config)
}

/// `{name}.client.toml` を元にリダイレクトを待ち受けるサーバーを起動する
/// 立てたら立てっぱなしなので、 `stop_server` を呼んで止めて欲しい
#[tauri::command(rename_all = "camelCase")]
async fn start_server(app: AppHandle, name: &str) -> Result<Url, String> {
    let config = app
        .open_client_config(name)
        .await
        .map_err(|e| e.to_string())?;

    let redirect_url = config.redirect_url.url().clone();

    let authorizer = OAuth2Authorizer::new(config);
    let authorize_url = authorizer.authorize_url().clone();

    let th = spawn_axum_server(redirect_url, app.clone(), name.to_string(), authorizer)
        .await
        .map_err(|e| e.to_string())?;

    let state = app.state::<AuthorizerThreadsState>();
    let mut threads = state.lock().await;
    threads.insert(name.to_string(), th);

    Ok(authorize_url)
}

/// `start_server` で立てたリダイレクトを待ち受けるサーバーを止める
/// `.with_graceful_shutdown` どこに行っちゃったんですか??
#[tauri::command(rename_all = "camelCase")]
async fn stop_server(app: AppHandle, name: &str) -> Result<(), String> {
    let state = app.state::<AuthorizerThreadsState>();
    let mut threads = state.lock().await;
    if let Some(thread) = threads.remove(name) {
        thread.abort();
        let _ = thread.await;
        log::info!("Aborted {name} thread.");
    }
    Ok(())
}

/// リダイレクトを待ち受ける axum のスレッドを保管しておく
#[derive(Default, Debug)]
pub struct AuthorizerThreadsState(Mutex<HashMap<String, JoinHandle<()>>>);

impl Deref for AuthorizerThreadsState {
    type Target = Mutex<HashMap<String, JoinHandle<()>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AuthorizerThreadsState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug)]
struct HttpState(String, AppHandle, Arc<OAuth2Authorizer>);

/// リダイレクトを受け付けるサーバーを tokio::spawn から起動する
/// TODO: graceful shutdown したい
async fn spawn_axum_server(
    redirect_url: Url,
    handle: AppHandle,
    name: String,
    authorizer: OAuth2Authorizer,
) -> Result<JoinHandle<()>> {
    let addrs = redirect_url.socket_addrs(|| None)?;
    let Some(server_addr) = addrs.first() else {
        bail!("Failed convert server addr from redirect_url.");
    };
    let tcp = TcpListener::bind(server_addr).await?;

    let path = redirect_url.path().to_string();

    let th = tokio::spawn(async move {
        let router = axum::Router::new()
            .route(&path, get(handle_get_exchange_code))
            .with_state(HttpState(name, handle, Arc::new(authorizer)));
        let _ = axum::serve(tcp, router.into_make_service()).await;
    });

    Ok(th)
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeTokenData {
    name: String,
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ExchangeQueryParams {
    pub code: String,
    pub state: String,
}

/// リダイレクトで受けた code, state を元に token を発行して AppHandle から emit する
async fn handle_get_exchange_code(
    State(HttpState(name, handle, authorizer)): State<HttpState>,
    Query(params): Query<ExchangeQueryParams>,
) -> impl IntoResponse {
    let v = match authorizer.exchange_code(&params.code, &params.state).await {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };
    let payload = ExchangeTokenData {
        name: name.clone(),
        access_token: v.access_token().secret().to_string(),
        refresh_token: v.refresh_token().map(|r| r.secret().to_string()),
    };
    if let Err(e) = handle.emit_all("token-response", payload) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    (StatusCode::OK, "OK".to_string())
}
