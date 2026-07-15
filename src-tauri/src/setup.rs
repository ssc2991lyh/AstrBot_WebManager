use std::sync::Arc;

use crate::commands;
use crate::config::{load_config, load_manifest, with_manifest_mut};
use crate::runtime::AppState;
use crate::utils::log_bus as log_channel;

/// Headless setup: no window/tray/autostart. Starts the instance monitor and
/// the two event forwarders (runtime events -> app-snapshot SSE, log bus ->
/// log-entry SSE), then restores any tracked instances if configured.
pub fn on_setup(state: Arc<AppState>) {
    state.process_manager.start_monitor();

    spawn_event_forwarder(state.clone());
    spawn_log_forwarder(state.clone());
    restore_instances(&state);
}

fn spawn_event_forwarder(state: Arc<AppState>) {
    let mut rx = state.process_manager.subscribe_runtime_events();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(_event) => {
                    let pm_clone = state.process_manager.clone();
                    match tokio::task::spawn_blocking(move || {
                        commands::build_app_snapshot_with(&pm_clone, load_config, load_manifest)
                    })
                    .await
                    {
                        Ok(Ok(snapshot)) => {
                            state.handle().emit("app-snapshot", &snapshot);
                        }
                        Ok(Err(e)) => log::warn!("Failed to build app snapshot for event: {}", e),
                        Err(e) => log::warn!("Snapshot task panicked: {}", e),
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("Runtime event listener lagged, skipped {} events", skipped);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn spawn_log_forwarder(state: Arc<AppState>) {
    let mut log_rx = log_channel::get_log_sender().subscribe();
    tokio::spawn(async move {
        loop {
            match log_rx.recv().await {
                Ok(entry) => {
                    state.handle().emit("log-entry", &entry);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn restore_instances(state: &AppState) {
    if let (Ok(cfg), Ok(manifest)) = (load_config(), load_manifest()) {
        if cfg.persist_instance_state && !manifest.tracked_instances_snapshot.is_empty() {
            let ids = manifest.tracked_instances_snapshot.clone();
            for id in &ids {
                let id = id.clone();
                let handle = state.handle();
                let pm = state.process_manager.clone();
                tokio::spawn(async move {
                    if let Err(e) = pm.start_instance(&id, handle).await {
                        log::error!("Failed to restore instance {}: {:?}", id, e);
                    }
                });
            }
            // Clear the snapshot after restoration attempt.
            let _ = with_manifest_mut(|manifest| {
                manifest.tracked_instances_snapshot.clear();
                Ok(())
            });
        }
    }
}
