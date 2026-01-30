//! CLI tool execution (custom scripts).

use std::path::Path;
use std::sync::Arc;

use tracing::warn;

use crate::llm::{FunctionDefinition, ToolDefinition};
use crate::sandbox::Sandbox;

use super::error::ToolError;
use super::executor::ToolResult;

/// Arguments for CLI tools.
#[derive(Default, serde::Deserialize)]
struct CliArgs {
    #[serde(default)]
    args: String,
}

/// Execute a CLI tool.
pub async fn execute(
    sandbox: &Arc<dyn Sandbox>,
    agent_dir: &Path,
    command: &str,
    arguments: &str,
) -> Result<ToolResult, ToolError> {
    // Parse arguments JSON
    let args: CliArgs = serde_json::from_str(arguments).unwrap_or_else(|e| {
        warn!(
            error = %e,
            arguments,
            "Failed to parse CLI tool arguments, using defaults"
        );
        CliArgs::default()
    });

    // Build the full command
    let full_command = if args.args.is_empty() {
        command.to_string()
    } else {
        format!("{} {}", command, args.args)
    };

    // Execute via sandbox (uses default timeout)
    let result = sandbox
        .exec(
            "bash",
            &["-c".to_string(), full_command],
            Some(agent_dir),
            None,
        )
        .await?;

    Ok(ToolResult::from_exec(result))
}

/// Generate a tool definition for a CLI tool.
pub fn definition(name: &str, description: Option<&str>) -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: name.to_string(),
            description: description
                .map(String::from)
                .unwrap_or_else(|| format!("CLI tool: {}", name)),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "args": {
                        "type": "string",
                        "description": "Command line arguments to pass to the tool"
                    }
                }
            })),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_with_description() {
        let def = definition("git-helper", Some("Run git commands"));
        assert_eq!(def.function.name, "git-helper");
        assert_eq!(def.function.description, "Run git commands");
    }

    #[test]
    fn definition_without_description() {
        let def = definition("my-tool", None);
        assert_eq!(def.function.name, "my-tool");
        assert_eq!(def.function.description, "CLI tool: my-tool");
    }
}
