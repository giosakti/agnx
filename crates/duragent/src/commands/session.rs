//! `duragent session` command implementations.

use std::path::Path;

use anyhow::{Context, Result};

use duragent::config::Config;
use duragent::launcher::{LaunchOptions, ensure_server_running};

/// List all sessions from the server.
pub async fn list(
    config_path: &str,
    agents_dir_override: Option<&Path>,
    server_url: Option<&str>,
) -> Result<()> {
    super::check_workspace(config_path)?;
    let config = Config::load(config_path).await?;

    let client = ensure_server_running(LaunchOptions {
        server_url,
        config_path: Path::new(config_path),
        config: &config,
        agents_dir: agents_dir_override,
    })
    .await
    .context("Failed to connect to server")?;

    let sessions = client.list_sessions().await?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!("{:<40} {:<20} {:<10}", "SESSION ID", "AGENT", "STATUS");
    println!("{:-<40} {:-<20} {:-<10}", "", "", "");

    for session in &sessions {
        println!(
            "{:<40} {:<20} {:<10}",
            session.session_id, session.agent, session.status
        );
    }

    Ok(())
}

/// Delete a session.
pub async fn delete(
    session_id: &str,
    config_path: &str,
    agents_dir_override: Option<&Path>,
    server_url: Option<&str>,
) -> Result<()> {
    super::check_workspace(config_path)?;
    let config = Config::load(config_path).await?;

    let client = ensure_server_running(LaunchOptions {
        server_url,
        config_path: Path::new(config_path),
        config: &config,
        agents_dir: agents_dir_override,
    })
    .await
    .context("Failed to connect to server")?;

    client
        .delete_session(session_id)
        .await
        .with_context(|| format!("Failed to delete session '{}'", session_id))?;

    println!("Deleted session {}", session_id);
    Ok(())
}
