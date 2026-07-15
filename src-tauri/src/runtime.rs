//! Headless runtime shim.
//!
//! Replaces Tauri's `AppHandle` / `State` / `Emitter` with a tiny in-process
//! event bus so the Launcher backend can run as a plain HTTP service (axum)
//! without any Tauri/WebKit/GTK dependency. Business logic in `commands.rs`
//! and the `instance`/`component`/`process` modules is untouched — they only
//! ever used `AppHandle` to call `.emit(event, payload)`, which this shim
//! forwards onto a `broadcast::Sender` consumed by the SSE endpoint.

use std::sync::RwLock;

use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::error::AppError;
use crate::process::ProcessManager;
use crate::utils::sync::{read_lock_recover, write_lock_recover};

/// An event destined for the browser (SSE). `event` is the logical name
/// (e.g. "app-snapshot", "log-entry", "deploy-progress") and `payload` is the
/// already-serialized JSON body.
#[derive(Clone, Debug, serde::Serialize)]
pub struct EmittedEvent {
    pub event: String,
    pub payload: Value,
}

/// Drop-in replacement for `tauri::AppHandle`.
///
/// Holds a clone of the global event broadcast sender. The only thing the
/// rest of the codebase ever did with an `AppHandle` was call `.emit(...)`,
/// which this implements as a method.
#[derive(Clone)]
pub struct AppHandle {
    event_tx: broadcast::Sender<EmittedEvent>,
}

impl AppHandle {
    /// Emit an event to every connected SSE client. Serialization failures
    /// are swallowed (logged via the payload being dropped) to keep the
    /// caller ergonomics identical to `tauri::Emitter::emit`.
    pub fn emit<S: Serialize>(&self, event: &str, payload: S) {
        match serde_json::to_value(&payload) {
            Ok(value) => {
                let _ = self.event_tx.send(EmittedEvent {
                    event: event.to_string(),
                    payload: value,
                });
            }
            Err(e) => log::warn!("Failed to serialize event '{}': {}", event, e),
        }
    }
}

/// Application state. Previously wrapped by `tauri::State<'_, AppState>`; now
/// owned directly by the axum router as an `Arc<AppState>`.
pub struct AppState {
    pub client: RwLock<Client>,
    pub process_manager: ProcessManager,
    event_tx: broadcast::Sender<EmittedEvent>,
}

impl AppState {
    pub fn new(client: Client) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            client: RwLock::new(client),
            process_manager: ProcessManager::new(),
            event_tx,
        }
    }

    /// Subscribe to the outbound event bus (used by the SSE endpoint).
    pub fn subscribe_events(&self) -> broadcast::Receiver<EmittedEvent> {
        self.event_tx.subscribe()
    }

    /// Construct an `AppHandle` bound to this state's event bus. Passed to
    /// commands that need to report progress (deploy/download/start/...).
    pub fn handle(&self) -> AppHandle {
        AppHandle {
            event_tx: self.event_tx.clone(),
        }
    }

    pub(crate) fn client(&self) -> Client {
        read_lock_recover(&self.client, "AppState.client").clone()
    }

    pub(crate) fn replace_client(&self, client: Client) {
        *write_lock_recover(&self.client, "AppState.client") = client;
    }
}

/// Helper so the dispatch layer can build an error from a missing field.
pub fn missing_arg(name: &str) -> AppError {
    AppError::other(format!("缺少参数: {name}"))
}
