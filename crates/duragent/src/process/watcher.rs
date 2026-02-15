//! Stream-based screen watcher for interactive tmux processes.
//!
//! Monitors the process log file size to detect when output stops flowing.
//! This is faster and cheaper than polling `capture-pane`:
//!
//! - Detection is based on output silence, not screen hash comparison
//! - During active output, only a `stat()` call per tick (no subprocess spawn)
//! - `capture-pane` is only called when silence triggers a callback

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::oneshot;
use tokio::time::Instant;
use tracing::debug;

use super::ProcessRegistryHandle;
/// Spawn a stream-based screen watcher for a tmux process.
///
/// Polls the log file size every `poll_interval`. When the file size
/// hasn't changed for `silence_timeout`, fires a screen-halted callback.
/// Does not re-fire until new output appears and then goes silent again.
pub fn spawn_screen_watcher(
    handle_id: String,
    poll_interval: Duration,
    silence_timeout: Duration,
    registry: ProcessRegistryHandle,
    cancel_rx: oneshot::Receiver<()>,
    log_path: PathBuf,
) {
    tokio::spawn(async move {
        debug!(handle = %handle_id, "Stream watcher started");

        let mut last_size: u64 = 0;
        let mut silence_start: Option<Instant> = None;
        let mut fired_for_silence = false;
        let mut interval = tokio::time::interval(poll_interval);
        let mut cancel_rx = cancel_rx;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !is_running(&registry, &handle_id) {
                        debug!(handle = %handle_id, "Process no longer running, stopping watcher");
                        break;
                    }

                    let current_size = match tokio::fs::metadata(&log_path).await {
                        Ok(m) => m.len(),
                        Err(_) => continue,
                    };

                    if current_size != last_size {
                        // Output is flowing — reset silence tracking
                        last_size = current_size;
                        silence_start = None;
                        fired_for_silence = false;

                    } else if !fired_for_silence {
                        // No new output — track silence duration
                        let start = silence_start.get_or_insert(Instant::now());
                        if start.elapsed() >= silence_timeout {
                            debug!(handle = %handle_id, "Output silent, firing callback");
                            registry.fire_screen_halted_callback(&handle_id).await;
                            fired_for_silence = true;
                        }
                    }
                }
                _ = &mut cancel_rx => {
                    debug!(handle = %handle_id, "Stream watcher cancelled");
                    break;
                }
            }
        }

        debug!(handle = %handle_id, "Stream watcher stopped");
    });
}

fn is_running(registry: &ProcessRegistryHandle, handle_id: &str) -> bool {
    registry
        .entries
        .get(handle_id)
        .is_some_and(|entry| !entry.meta.status.is_terminal())
}
