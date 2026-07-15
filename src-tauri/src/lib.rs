mod archive;
mod backup;
mod commands;
mod component;
mod config;
mod download;
mod error;
pub use error::{AppError, ErrorKind, Result};
mod github;
mod instance;
mod migration;
mod network_config;
mod platform;
mod process;
mod runtime;
mod setup;
mod utils;
mod validation;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::Router;
use futures_util::StreamExt;
use log::LevelFilter;
use reqwest::Client;
use serde::Serialize;
use serde_json::{Value, json};
use tokio_stream::wrappers::BroadcastStream;

use crate::commands::{LockCheckTarget, SystemdStatus};
use crate::config::{load_config, load_manifest, with_manifest_mut, ThemePreference};
use crate::github::GitHubRelease;
use crate::instance::RepairPreserveScope;
use crate::runtime::{AppState, missing_arg};
use crate::utils::log_bus::{self, LogEntry};

const DEFAULT_PORT: u16 = 6190;

#[allow(clippy::expect_used)]
pub fn run() {
    crate::utils::paths::ensure_data_dirs().expect("Failed to create data directories");
    migration::run_startup_migrations();
    github::init_releases_cache();

    // Logger: forward to stdout + in-process channel (consumed by SSE endpoint).
    let dispatch_sender = log_bus::init_log_channel();
    fern::Dispatch::new()
        .chain(fern::Output::call(move |record| {
            let _ = dispatch_sender.send(LogEntry {
                source: "system".to_string(),
                level: record.level().to_string().to_lowercase(),
                message: record.args().to_string(),
                timestamp: chrono::Local::now().to_rfc3339(),
            });
        }))
        .chain(std::io::stdout())
        .level(LevelFilter::Debug)
        .apply()
        .expect("Failed to initialize logger");

    let config = load_config().expect("Failed to load config");
    let client = network_config::build_http_client_from_config(config.as_ref())
        .unwrap_or_else(|e| {
            log::warn!("Proxy HTTP client build failed, using fallback: {}", e);
            Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create fallback HTTP client")
        });

    let state = Arc::new(AppState::new(client));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async move {
        // on_setup must run inside the runtime — it calls tokio::spawn
        setup::on_setup(state.clone());
        spawn_graceful_shutdown(state.clone());

        let app = build_router(state.clone());
        let port: u16 = std::env::var("ASTRBOT_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_PORT);
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .unwrap_or_else(|e| panic!("Failed to bind {}: {}", addr, e));
        log::info!("AstrBot HTTP API listening on http://0.0.0.0:{}", port);
        axum::serve(listener, app)
            .await
            .expect("Server error");
    });
}

/// Persist tracked instance IDs and stop everything on Ctrl-C.
fn spawn_graceful_shutdown(state: Arc<AppState>) {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            log::info!("Received shutdown signal, stopping all instances...");
            if let Ok(cfg) = load_config() {
                if cfg.persist_instance_state {
                    let ids = state.process_manager.get_active_ids();
                    let _ = with_manifest_mut(|m| {
                        m.tracked_instances_snapshot = ids;
                        Ok(())
                    });
                }
            }
            state.process_manager.stop_all_blocking();
        }
    });
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/events", get(sse_handler))
        .route("/api/{cmd}", post(api_handler))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state)
}

async fn api_handler(
    Path(cmd): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(args): Json<Value>,
) -> Response {
    match dispatch(&cmd, args, &state).await {
        Ok(value) => Json(value).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn sse_handler(State(state): State<Arc<AppState>>) -> Response {
    let rx = state.subscribe_events();
    let stream = BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(evt) => match serde_json::to_string(&evt.payload) {
                Ok(data) => Some(Ok::<Event, std::convert::Infallible>(Event::default().event(evt.event).data(data))),
                Err(_) => None,
            },
            Err(_) => None,
        }
    });
    Sse::new(stream).into_response()
}

// ---- dispatch ----

fn arg_str(args: &Value, k: &str) -> Result<String> {
    args.get(k)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| missing_arg(k))
}
fn arg_bool(args: &Value, k: &str) -> Result<bool> {
    args.get(k)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| missing_arg(k))
}
fn arg_u16(args: &Value, k: &str) -> Result<u16> {
    args.get(k)
        .and_then(|v| v.as_u64())
        .map(|n| n as u16)
        .ok_or_else(|| missing_arg(k))
}
fn arg_opt_str(args: &Value, k: &str) -> Option<String> {
    args.get(k).and_then(|v| v.as_str()).map(String::from)
}
fn arg_opt_u16(args: &Value, k: &str) -> Option<u16> {
    args.get(k).and_then(|v| v.as_u64()).map(|n| n as u16)
}
fn to_json<T: Serialize>(t: T) -> Result<Value> {
    serde_json::to_value(t).map_err(|e| AppError::other(format!("serialize error: {e}")))
}
fn from_arg<T: serde::de::DeserializeOwned>(args: &Value, k: &str) -> Result<T> {
    let v = args.get(k).cloned().ok_or_else(|| missing_arg(k))?;
    serde_json::from_value(v).map_err(|e| AppError::other(format!("arg {k}: {e}")))
}

#[allow(clippy::too_many_lines)]
async fn dispatch(cmd: &str, args: Value, state: &AppState) -> Result<Value> {
    let h = state.handle();
    match cmd {
        "get_app_snapshot" => to_json(commands::get_app_snapshot(state).await?),
        "rebuild_app_snapshot" => to_json(commands::rebuild_app_snapshot(state).await?),
        "get_version" => to_json(commands::get_version()),
        "get_systemd_status" => to_json(commands::get_systemd_status().await?),
        "compare_versions" => {
            let a = arg_str(&args, "a")?;
            let b = arg_str(&args, "b")?;
            to_json(commands::compare_versions(a, b))
        }
        "save_github_proxy" => {
            let v = arg_str(&args, "github_proxy")?;
            to_json(commands::save_github_proxy(v, state).await?)
        }
        "save_proxy" => {
            let u = arg_str(&args, "proxy_url")?;
            let p = arg_str(&args, "proxy_port")?;
            let uu = arg_str(&args, "proxy_username")?;
            let pp = arg_str(&args, "proxy_password")?;
            to_json(commands::save_proxy(u, p, uu, pp, state).await?)
        }
        "save_pypi_mirror" => {
            let v = arg_str(&args, "pypi_mirror")?;
            to_json(commands::save_pypi_mirror(v, state).await?)
        }
        "save_mainland_acceleration" => {
            let v = arg_bool(&args, "mainland_acceleration")?;
            to_json(commands::save_mainland_acceleration(v, state).await?)
        }
        "save_use_uv_for_deps" => {
            let v = arg_bool(&args, "use_uv_for_deps")?;
            to_json(commands::save_use_uv_for_deps(v).await?)
        }
        "save_close_to_tray" => {
            let v = arg_bool(&args, "close_to_tray")?;
            to_json(commands::save_close_to_tray(v).await?)
        }
        "save_autostart_minimize_to_tray" => {
            let v = arg_bool(&args, "autostart_minimize_to_tray")?;
            to_json(commands::save_autostart_minimize_to_tray(v).await?)
        }
        "save_check_instance_update" => {
            let v = arg_bool(&args, "check_instance_update")?;
            to_json(commands::save_check_instance_update(v).await?)
        }
        "save_persist_instance_state" => {
            let v = arg_bool(&args, "persist_instance_state")?;
            to_json(commands::save_persist_instance_state(v).await?)
        }
        "save_ignore_external_path" => {
            let v = arg_bool(&args, "ignore_external_path")?;
            to_json(commands::save_ignore_external_path(v).await?)
        }
        "save_lock_check_extension_whitelist" => {
            let v = arg_bool(&args, "lock_check_extension_whitelist")?;
            to_json(commands::save_lock_check_extension_whitelist(v).await?)
        }
        "save_theme_preference" => {
            let v: ThemePreference = from_arg(&args, "theme_preference")?;
            to_json(commands::save_theme_preference(v).await?)
        }
        "save_nodejs_mirror" => {
            let v = arg_str(&args, "nodejs_mirror")?;
            to_json(commands::save_nodejs_mirror(v).await?)
        }
        "save_npm_registry" => {
            let v = arg_str(&args, "npm_registry")?;
            to_json(commands::save_npm_registry(v).await?)
        }
        "install_component" => {
            let id = arg_str(&args, "component_id")?;
            to_json(commands::install_component(h.clone(), state, id).await?)
        }
        "reinstall_component" => {
            let id = arg_str(&args, "component_id")?;
            to_json(commands::reinstall_component(h.clone(), state, id).await?)
        }
        "uninstall_component" => {
            let id = arg_str(&args, "component_id")?;
            to_json(commands::uninstall_component(h.clone(), state, id).await?)
        }
        "fetch_releases" => {
            let fr = args.get("force_refresh").and_then(|v| v.as_bool());
            to_json(commands::fetch_releases(state, fr).await?)
        }
        "fetch_launcher_release_notes" => {
            let v = arg_str(&args, "version")?;
            to_json(commands::fetch_launcher_release_notes(state, v).await?)
        }
        "install_version" => {
            let rel: GitHubRelease = from_arg(&args, "release")?;
            to_json(commands::install_version(h.clone(), state, rel).await?)
        }
        "uninstall_version" => {
            let v = arg_str(&args, "version")?;
            to_json(commands::uninstall_version(v).await?)
        }
        "check_lock" => {
            let target: LockCheckTarget = from_arg(&args, "target")?;
            let iid = arg_opt_str(&args, "instance_id");
            let bp = arg_opt_str(&args, "backup_path");
            to_json(commands::check_lock(target, iid, bp, state).await?)
        }
        "clear_instance_data" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::clear_instance_data(id, state).await?)
        }
        "clear_instance_venv" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::clear_instance_venv(id, state).await?)
        }
        "clear_pycache" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::clear_pycache(id, state).await?)
        }
        "repair_instance" => {
            let id = arg_str(&args, "instance_id")?;
            let ps: RepairPreserveScope = from_arg(&args, "preserve_scope")?;
            to_json(commands::repair_instance(h.clone(), id, ps, state).await?)
        }
        "rebuild_instance_manifest" => {
            to_json(commands::rebuild_instance_manifest(state).await?)
        }
        "create_instance" => {
            let n = arg_str(&args, "name")?;
            let v = arg_str(&args, "version")?;
            let p = arg_u16(&args, "port")?;
            to_json(commands::create_instance(n, v, p).await?)
        }
        "delete_instance" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::delete_instance(id, state).await?)
        }
        "update_instance" => {
            let id = arg_str(&args, "instance_id")?;
            let n = arg_opt_str(&args, "name");
            let v = arg_opt_str(&args, "version");
            let ho = arg_opt_str(&args, "host");
            let p = arg_opt_u16(&args, "port");
            to_json(commands::update_instance(h.clone(), id, n, v, ho, p, state).await?)
        }
        "start_instance" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::start_instance(h.clone(), id, state).await?)
        }
        "stop_instance" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::stop_instance(id, state).await?)
        }
        "restart_instance" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::restart_instance(h.clone(), id, state).await?)
        }
        "get_instance_port" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::get_instance_port(id, state).await?)
        }
        "create_backup" => {
            let id = arg_str(&args, "instance_id")?;
            to_json(commands::create_backup(id, state).await?)
        }
        "restore_backup" => {
            let bp = arg_str(&args, "backup_path")?;
            to_json(commands::restore_backup(bp, state).await?)
        }
        "delete_backup" => {
            let bp = arg_str(&args, "backup_path")?;
            to_json(commands::delete_backup(bp).await?)
        }
        "set_systemd_enabled" => {
            let e = arg_bool(&args, "enable")?;
            to_json(commands::set_systemd_enabled(e).await?)
        }
        "restart_manager" => to_json(commands::restart_manager().await?),
        "stop_manager" => to_json(commands::stop_manager().await?),
        _ => Err(AppError::other(format!("未知命令: {cmd}"))),
    }
}
