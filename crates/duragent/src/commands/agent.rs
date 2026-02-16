//! `duragent agent create` command implementation.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use duragent::config::{self, Config, DEFAULT_AGENTS_DIR, DEFAULT_WORKSPACE};

use super::init::{
    DEFAULT_MODEL, DEFAULT_PROVIDER, create_agent_files, credential_hint, print_file_summary,
    prompt_with_default,
};

pub async fn create(
    config_path: &str,
    agent_name: &str,
    provider: Option<String>,
    model: Option<String>,
    no_interactive: bool,
) -> Result<()> {
    super::check_workspace(config_path)?;

    // Resolve agents dir from config
    let config_path_ref = Path::new(config_path);
    let config = Config::load(config_path).await?;
    let workspace_raw = config
        .workspace
        .as_deref()
        .unwrap_or(Path::new(DEFAULT_WORKSPACE));
    let workspace = config::resolve_path(config_path_ref, workspace_raw);
    let agents_dir = config
        .agents_dir
        .as_ref()
        .map(|p| config::resolve_path(config_path_ref, p))
        .unwrap_or_else(|| workspace.join(DEFAULT_AGENTS_DIR));

    // Check agent doesn't already exist
    let agent_dir = agents_dir.join(agent_name);
    if agent_dir.exists() {
        bail!(
            "Agent '{}' already exists at '{}'.\n\
             Remove it first or choose a different name.",
            agent_name,
            agent_dir.display()
        );
    }

    let provider = match provider {
        Some(p) => p,
        None if no_interactive => DEFAULT_PROVIDER.to_string(),
        None => prompt_with_default(
            "LLM provider (anthropic, openrouter, openai, ollama)",
            DEFAULT_PROVIDER,
        )?,
    };

    let model = match model {
        Some(m) => m,
        None if no_interactive => DEFAULT_MODEL.to_string(),
        None => prompt_with_default("Model name", DEFAULT_MODEL)?,
    };

    let (created, skipped) = create_agent_files(&agents_dir, agent_name, &provider, &model).await?;

    // Strip to workspace-relative paths for display (same as init)
    let cwd = std::env::current_dir().unwrap_or_default();
    let strip = |paths: Vec<PathBuf>| -> Vec<PathBuf> {
        paths
            .into_iter()
            .map(|p| p.strip_prefix(&cwd).map(|r| r.to_path_buf()).unwrap_or(p))
            .collect()
    };
    print_file_summary(&strip(created), &strip(skipped));

    println!();
    println!("Agent '{agent_name}' created!");
    if let Some(hint) = credential_hint(&provider) {
        println!("  {hint}");
    }
    println!("  Run: duragent chat --agent {agent_name}");

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::init::TEMPLATE_AGENT_YAML;
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    /// Set up a minimal workspace (duragent.yaml + .duragent/ directory).
    async fn setup_workspace(root: &Path) {
        fs::create_dir_all(root.join(".duragent/agents"))
            .await
            .unwrap();
        fs::write(
            root.join("duragent.yaml"),
            "server:\n  host: 127.0.0.1\n  port: 8080\n",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_create_happy_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        setup_workspace(root).await;

        let config_path = root.join("duragent.yaml");
        create(
            config_path.to_str().unwrap(),
            "new-bot",
            Some("openrouter".to_string()),
            Some("anthropic/claude-sonnet-4".to_string()),
            true,
        )
        .await
        .unwrap();

        let agent_dir = root.join(".duragent/agents/new-bot");
        assert!(agent_dir.join("agent.yaml").exists());
        assert!(agent_dir.join("policy.yaml").exists());
        assert!(agent_dir.join("SOUL.md").exists());
        assert!(agent_dir.join("SYSTEM_PROMPT.md").exists());

        let agent = std::fs::read_to_string(agent_dir.join("agent.yaml")).unwrap();
        assert!(agent.contains("name: new-bot"));
        assert!(agent.contains("provider: openrouter"));
        assert!(agent.contains("name: anthropic/claude-sonnet-4"));
    }

    #[tokio::test]
    async fn test_create_idempotent() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        setup_workspace(root).await;

        let config_path = root.join("duragent.yaml");
        let config = config_path.to_str().unwrap();

        // First create succeeds
        create(
            config,
            "bot-a",
            Some("openrouter".to_string()),
            Some("model-1".to_string()),
            true,
        )
        .await
        .unwrap();

        // Second create with same name fails (agent already exists)
        let err = create(
            config,
            "bot-a",
            Some("openrouter".to_string()),
            Some("model-1".to_string()),
            true,
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_no_workspace() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let config_path = root.join("duragent.yaml");

        let err = create(
            config_path.to_str().unwrap(),
            "bot",
            Some("openrouter".to_string()),
            Some("model".to_string()),
            true,
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("No duragent workspace found"));
    }

    #[tokio::test]
    async fn test_create_custom_provider_and_model() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        setup_workspace(root).await;

        let config_path = root.join("duragent.yaml");
        create(
            config_path.to_str().unwrap(),
            "custom-bot",
            Some("anthropic".to_string()),
            Some("claude-sonnet-4-20250514".to_string()),
            true,
        )
        .await
        .unwrap();

        let agent =
            std::fs::read_to_string(root.join(".duragent/agents/custom-bot/agent.yaml")).unwrap();
        assert!(agent.contains("provider: anthropic"));
        assert!(agent.contains("name: claude-sonnet-4-20250514"));
    }

    #[tokio::test]
    async fn test_create_defaults_match_init() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        setup_workspace(root).await;

        let config_path = root.join("duragent.yaml");
        create(
            config_path.to_str().unwrap(),
            "match-bot",
            Some("openrouter".to_string()),
            Some("anthropic/claude-sonnet-4".to_string()),
            true,
        )
        .await
        .unwrap();

        // The generated agent.yaml should match the template with placeholders replaced
        let expected = TEMPLATE_AGENT_YAML
            .replace("{name}", "match-bot")
            .replace("{provider}", "openrouter")
            .replace("{model}", "anthropic/claude-sonnet-4");

        let actual =
            std::fs::read_to_string(root.join(".duragent/agents/match-bot/agent.yaml")).unwrap();
        assert_eq!(actual, expected);
    }
}
