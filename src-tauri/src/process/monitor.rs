//! Runtime monitoring: liveness probes and state reconciliation.
//!
//! All evaluation functions are standalone — they take snapshot data and return results.
//! The monitor task (spawned by ProcessManager) calls `poll_instances` on a timer.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use super::control::is_expected_process_alive;
use super::manager::{InstanceEntry, ProcessState, Slot};
use super::{InstanceState, RuntimeEvent};
use crate::utils::sync::lock_mutex_recover;

#[cfg(target_os = "windows")]
use super::control::is_process_alive;
#[cfg(target_os = "windows")]
use super::win_api::get_pid_on_port;
#[cfg(target_os = "windows")]
use super::ALIVE_EXIT_THRESHOLD;

/// Snapshot of a single live instance for evaluation.
struct LiveSnapshot {
    id: String,
    port: u16,
    pid: u32,
    executable_path: PathBuf,
    dashboard_enabled: bool,
    alive_failure_count: u32,
    next_alive_check_at: Option<Instant>,
}

/// Outcome of evaluating a single instance during monitoring.
enum MonitorOutcome {
    /// The process is dead — its slot should be removed.
    Dead { id: String },
    /// The process is alive — update its fields.
    Alive {
        id: String,
        /// Always 0 / `None` on non-Windows (no retry mechanism).
        alive_failure_count: u32,
        next_alive_check_at: Option<Instant>,
        new_pid: Option<u32>,
    },
}

/// Entry point called by the monitor task on each tick.
///
/// Snap-then-apply: locks briefly to snapshot, evaluates with no
/// lock held, then locks again to apply results.
pub(super) fn poll_instances(
    state: &Mutex<ProcessState>,
    runtime_events: &tokio::sync::broadcast::Sender<RuntimeEvent>,
) {
    // Phase 1: lock → snapshot Live slots → unlock
    let entries: Vec<LiveSnapshot> = {
        let state = lock_mutex_recover(state, "ProcessState");
        if state.shutting_down {
            return;
        }
        state
            .slots
            .iter()
            .filter_map(|(id, entry)| {
                if let Some(Slot::Live(p)) = &entry.slot {
                    Some(LiveSnapshot {
                        id: id.clone(),
                        port: p.port,
                        pid: p.pid,
                        executable_path: p.executable_path.clone(),
                        dashboard_enabled: p.dashboard_enabled,
                        alive_failure_count: p.alive_failure_count,
                        next_alive_check_at: p.next_alive_check_at,
                    })
                } else {
                    None
                }
            })
            .collect()
    };

    if entries.is_empty() {
        return;
    }

    // Phase 2: evaluate liveness
    let outcomes = evaluate_instances(&entries);

    // Phase 3: lock → apply outcomes → unlock → collect events
    let events = {
        let mut state = lock_mutex_recover(state, "ProcessState");
        apply_outcomes(&mut state, &outcomes)
    };

    // Phase 4: emit events (outside lock)
    for (id, new_state) in events {
        let _ = runtime_events.send(RuntimeEvent {
            instance_id: id,
            state: new_state,
        });
    }
}

/// Evaluate all instances via liveness probes.
fn evaluate_instances(entries: &[LiveSnapshot]) -> Vec<MonitorOutcome> {
    let now = Instant::now();
    let mut outcomes = Vec::new();

    for entry in entries {
        let Some(outcome) = evaluate_liveness(entry, now) else {
            outcomes.push(MonitorOutcome::Dead {
                id: entry.id.clone(),
            });
            continue;
        };

        outcomes.push(outcome);
    }

    outcomes
}

/// Apply monitor outcomes to the live process state. Returns events to emit.
fn apply_outcomes(
    state: &mut ProcessState,
    outcomes: &[MonitorOutcome],
) -> Vec<(String, InstanceState)> {
    let mut events = Vec::new();

    for outcome in outcomes {
        match outcome {
            MonitorOutcome::Dead { id } => {
                // Only remove slots still in Live state — another lifecycle
                // method may have transitioned the slot in the meantime.
                if matches!(
                    state.slots.get(id),
                    Some(InstanceEntry {
                        slot: Some(Slot::Live(_)),
                        ..
                    })
                ) {
                    state.slots.remove(id);
                    log::info!("Removed dead process tracking entry for instance {}", id);
                    events.push((id.clone(), InstanceState::Stopped));
                }
            }
            MonitorOutcome::Alive {
                id,
                alive_failure_count,
                next_alive_check_at,
                new_pid,
            } => {
                if let Some(InstanceEntry {
                    slot: Some(Slot::Live(p)),
                    ..
                }) = state.slots.get_mut(id)
                {
                    p.alive_failure_count = *alive_failure_count;
                    p.next_alive_check_at = *next_alive_check_at;
                    if let Some(new_pid) = new_pid {
                        log::info!(
                            "Instance {} PID updated: {} -> {} (port {})",
                            id,
                            p.pid,
                            new_pid,
                            p.port
                        );
                        p.pid = *new_pid;
                    }
                }
            }
        }
    }

    events
}

// -- Liveness evaluation ------------------------------------------------------

/// Evaluate liveness for a single instance. Returns `None` if the process is
/// dead (slot should be removed), `Some(MonitorOutcome::Alive { .. })` if alive.
///
/// Platform-specific probing is delegated to [`probe_liveness`], which returns
/// `None` for terminal death or `Some((alive_failure_count, next_alive_check_at,
/// new_pid))` when the process is alive or retriable.
fn evaluate_liveness(entry: &LiveSnapshot, now: Instant) -> Option<MonitorOutcome> {
    // Backoff: not yet time to probe — preserve previous counters.
    // On non-Windows this is always a no-op (next_alive_check_at is always None).
    if entry.dashboard_enabled {
        if let Some(next_at) = entry.next_alive_check_at {
            if now < next_at {
                return Some(MonitorOutcome::Alive {
                    id: entry.id.clone(),
                    alive_failure_count: entry.alive_failure_count,
                    next_alive_check_at: entry.next_alive_check_at,
                    new_pid: None,
                });
            }
        }
    }

    let (alive_failure_count, next_alive_check_at, new_pid) = probe_liveness(entry, now)?;

    // If dashboard disabled and we are in retry mode, treat as dead.
    if !entry.dashboard_enabled && next_alive_check_at.is_some() {
        return None;
    }

    Some(MonitorOutcome::Alive {
        id: entry.id.clone(),
        alive_failure_count,
        next_alive_check_at,
        new_pid,
    })
}

// -- Platform-specific liveness probing ---------------------------------------

/// Returns `None` for terminal death, or `Some((alive_failure_count,
/// next_alive_check_at, new_pid))` when the process is alive or retriable.
#[cfg(target_os = "windows")]
fn probe_liveness(
    entry: &LiveSnapshot,
    now: Instant,
) -> Option<(u32, Option<Instant>, Option<u32>)> {
    if is_expected_process_alive(entry.pid, &entry.executable_path) {
        return Some((0, None, None));
    }

    // PID check failed — try port-based PID discovery.
    if let Some(new_pid) = get_pid_on_port(entry.port) {
        if new_pid == entry.pid {
            if entry.alive_failure_count == 0 {
                log::warn!(
                    "Instance {} liveness probe failed for PID {}, but port {} still resolves to the same PID",
                    entry.id,
                    entry.pid,
                    entry.port
                );
            } else {
                log::debug!(
                    "Instance {} still resolves port {} to PID {} while liveness probe remains failed",
                    entry.id,
                    entry.port,
                    entry.pid
                );
            }
            // Fall through to failure handling below.
        } else if is_expected_process_alive(new_pid, &entry.executable_path) {
            return Some((0, None, Some(new_pid)));
        } else if is_process_alive(new_pid) {
            log::warn!(
                "Instance {} rejected PID update {} -> {}: executable path mismatch",
                entry.id,
                entry.pid,
                new_pid
            );
        } else {
            log::debug!(
                "Instance {} observed transient PID {} on port {}, but process was not alive during validation",
                entry.id,
                new_pid,
                entry.port
            );
        }
    }

    // Liveness probe failed — apply retry/threshold logic.
    let new_count = entry.alive_failure_count + 1;

    if new_count >= ALIVE_EXIT_THRESHOLD {
        log::warn!(
            "Instance {} liveness probe failed {} times, treating process as exited",
            entry.id,
            new_count
        );
        None
    } else {
        let interval = super::MONITOR_INTERVAL;
        log::debug!(
            "Instance {} liveness probe failed (count: {}), retry in {:?}",
            entry.id,
            new_count,
            interval
        );
        Some((new_count, Some(now + interval), None))
    }
}

#[cfg(not(target_os = "windows"))]
fn probe_liveness(
    entry: &LiveSnapshot,
    _now: Instant,
) -> Option<(u32, Option<Instant>, Option<u32>)> {
    if is_expected_process_alive(entry.pid, &entry.executable_path) {
        Some((0, None, None))
    } else {
        None
    }
}
