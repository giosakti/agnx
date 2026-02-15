//! tmux integration helpers.
//!
//! All functions shell out via `tokio::process::Command`.

use std::path::Path;

use tokio::process::Command;
use tracing::{debug, warn};

/// Default terminal width for tmux sessions.
const DEFAULT_WIDTH: u16 = 200;
/// Default terminal height for tmux sessions.
const DEFAULT_HEIGHT: u16 = 50;

/// Check if tmux is available on the system.
pub async fn detect_tmux() -> bool {
    match Command::new("tmux").arg("-V").output().await {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                debug!(version = %version.trim(), "tmux detected");
                true
            } else {
                debug!("tmux -V returned non-zero");
                false
            }
        }
        Err(_) => {
            debug!("tmux not found in PATH");
            false
        }
    }
}

/// Create a new tmux session running the given command.
///
/// For non-interactive processes the command is wrapped with `tee` to capture
/// output and the exit code.  For interactive processes (`interactive = true`)
/// the command runs directly so the PTY is preserved (programs like Claude Code
/// check `isatty`), and `pipe-pane` streams output to the log file instead.
pub async fn create_session(
    name: &str,
    command: &str,
    log_path: &Path,
    cwd: Option<&str>,
    interactive: bool,
) -> std::io::Result<()> {
    let log_str = log_path.to_string_lossy();
    // Escape characters special inside double quotes ($, `, \, ") so that
    // workspace paths containing these chars aren't interpreted by the shell.
    let escaped_log = log_str
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");

    let session_command = if interactive {
        // Run the command directly so the tmux PTY is connected to it.
        // Append an EXIT_CODE marker after the command finishes so the
        // monitor can detect the real exit code (pipe-pane only captures
        // output, not exit status).
        format!(
            "bash -c '{}; echo EXIT_CODE:$? >> \"{}\"'",
            command.replace('\'', "'\\''"),
            escaped_log,
        )
    } else {
        // Wrap command to capture output and exit code.
        // pipefail ensures $? reflects the command's exit code, not tee's.
        format!(
            "bash -c 'set -o pipefail; {} 2>&1 | tee \"{}\"; echo EXIT_CODE:$? >> \"{}\"'",
            command.replace('\'', "'\\''"),
            escaped_log,
            escaped_log,
        )
    };

    let mut cmd = Command::new("tmux");
    cmd.args(["new-session", "-d", "-s", name]);
    cmd.args([
        "-x",
        &DEFAULT_WIDTH.to_string(),
        "-y",
        &DEFAULT_HEIGHT.to_string(),
    ]);

    // -c must come BEFORE the positional shell-command argument.
    // tmux stops parsing options at the first non-option arg, so putting
    // -c after session_command causes it to be treated as part of the command.
    if let Some(dir) = cwd {
        cmd.args(["-c", dir]);
    }

    cmd.arg(&session_command);

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!(
            "tmux new-session failed: {}",
            stderr.trim()
        )));
    }

    // For interactive sessions, use pipe-pane to log output without breaking the PTY.
    if interactive {
        let pipe_output = Command::new("tmux")
            .args([
                "pipe-pane",
                "-t",
                name,
                &format!("cat >> \"{}\"", escaped_log),
            ])
            .output()
            .await?;
        if !pipe_output.status.success() {
            let stderr = String::from_utf8_lossy(&pipe_output.stderr);
            warn!(session = %name, error = %stderr.trim(), "tmux pipe-pane failed, logging disabled");
        }
    }

    Ok(())
}

/// Check if a tmux session exists.
pub async fn has_session(name: &str) -> bool {
    match Command::new("tmux")
        .args(["has-session", "-t", name])
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Capture the current content of a tmux pane.
pub async fn capture_pane(name: &str) -> std::io::Result<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", name, "-p"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!(
            "tmux capture-pane failed: {}",
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Send keystrokes to a tmux session.
///
/// The `keys` string is split on whitespace so that each token is passed as a
/// separate argument to `tmux send-keys`.  This means `"Down Enter"` sends two
/// keys (arrow-down then enter) instead of the literal text `"Down Enter"`.
///
/// When multiple keys are present, each key is sent as a separate `tmux
/// send-keys` call with a 50ms delay between them. This gives TUI applications
/// (like Claude Code's permission dialogs) time to process each keystroke
/// before the next one arrives.
///
/// To send literal text that contains spaces, use the `-l` flag via
/// [`send_literal`] instead.
///
/// If `press_enter` is true, an extra `Enter` keystroke is appended.
pub async fn send_keys(name: &str, keys: &str, press_enter: bool) -> std::io::Result<()> {
    let mut parts: Vec<&str> = keys.split_whitespace().collect();
    if press_enter {
        parts.push("Enter");
    }

    // Single key: send directly (no delay needed)
    if parts.len() <= 1 {
        let mut args: Vec<&str> = vec!["send-keys", "-t", name];
        args.extend(&parts);
        let output = Command::new("tmux").args(&args).output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(std::io::Error::other(format!(
                "tmux send-keys failed: {}",
                stderr.trim()
            )));
        }
        return Ok(());
    }

    // Multiple keys: send each individually with a delay between them
    for (i, key) in parts.iter().enumerate() {
        let output = Command::new("tmux")
            .args(["send-keys", "-t", name, key])
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(std::io::Error::other(format!(
                "tmux send-keys '{}' failed: {}",
                key,
                stderr.trim()
            )));
        }
        // Delay between keys (not after the last one)
        if i < parts.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    Ok(())
}

/// Send literal text to a tmux session (spaces are preserved, not split).
///
/// Uses `tmux send-keys -l` which treats the entire string as literal input.
/// If `press_enter` is true, an `Enter` keystroke is appended after the text.
pub async fn send_literal(name: &str, text: &str, press_enter: bool) -> std::io::Result<()> {
    let output = Command::new("tmux")
        .args(["send-keys", "-t", name, "-l", text])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!(
            "tmux send-keys -l failed: {}",
            stderr.trim()
        )));
    }

    // -l doesn't support appending Enter in the same call, so send it separately.
    if press_enter {
        let enter_output = Command::new("tmux")
            .args(["send-keys", "-t", name, "Enter"])
            .output()
            .await?;
        if !enter_output.status.success() {
            let stderr = String::from_utf8_lossy(&enter_output.stderr);
            return Err(std::io::Error::other(format!(
                "tmux send-keys Enter failed: {}",
                stderr.trim()
            )));
        }
    }

    Ok(())
}

/// Kill a tmux session.
pub async fn kill_session(name: &str) -> std::io::Result<()> {
    let output = Command::new("tmux")
        .args(["kill-session", "-t", name])
        .output()
        .await;

    match output {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            warn!(session = %name, error = %stderr.trim(), "tmux kill-session failed");
        }
        Err(e) => {
            warn!(session = %name, error = %e, "tmux kill-session failed");
        }
        _ => {}
    }

    Ok(())
}

/// Parse exit code from the last line of a log file.
///
/// Looks for `EXIT_CODE:<n>` pattern written by our tmux wrapper.
pub fn parse_exit_code_from_log(content: &str) -> Option<i32> {
    for line in content.lines().rev() {
        if let Some(code_str) = line.strip_prefix("EXIT_CODE:")
            && let Ok(code) = code_str.trim().parse::<i32>()
        {
            return Some(code);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exit_code_success() {
        let log = "some output\nmore output\nEXIT_CODE:0\n";
        assert_eq!(parse_exit_code_from_log(log), Some(0));
    }

    #[test]
    fn parse_exit_code_failure() {
        let log = "error output\nEXIT_CODE:1\n";
        assert_eq!(parse_exit_code_from_log(log), Some(1));
    }

    #[test]
    fn parse_exit_code_missing() {
        let log = "some output\nno exit code here\n";
        assert_eq!(parse_exit_code_from_log(log), None);
    }

    #[test]
    fn parse_exit_code_multiple_takes_last() {
        let log = "EXIT_CODE:0\nmore output\nEXIT_CODE:1\n";
        assert_eq!(parse_exit_code_from_log(log), Some(1));
    }
}
