//! Tool executor for running tools in agentic workflows.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs;
use tokio::sync::RwLock;

use super::bash;
use super::cli;
use super::error::ToolError;
use super::notify::send_notification;
use crate::agent::{NotifyConfig, PolicyDecision, ToolConfig, ToolPolicy, ToolType};
use crate::llm::{FunctionDefinition, ToolCall, ToolDefinition};
use crate::sandbox::{ExecResult, Sandbox};

// ============================================================================
// Types
// ============================================================================

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Whether the tool succeeded.
    pub success: bool,
    /// Content for LLM consumption.
    pub content: String,
}

impl ToolResult {
    /// Build a ToolResult from sandbox execution output.
    pub fn from_exec(result: ExecResult) -> Self {
        let mut content = String::new();

        if !result.stdout.is_empty() {
            content.push_str(&result.stdout);
        }
        if !result.stderr.is_empty() {
            if !content.is_empty() {
                content.push_str("\n--- stderr ---\n");
            }
            content.push_str(&result.stderr);
        }
        if content.is_empty() {
            content = format!("Command completed with exit code {}", result.exit_code);
        }

        Self {
            success: result.exit_code == 0,
            content,
        }
    }
}

// ============================================================================
// Executor
// ============================================================================

/// Executor for running tools.
pub struct ToolExecutor {
    /// Tool configurations by name.
    tools: HashMap<String, ToolConfig>,
    /// Sandbox for executing commands.
    sandbox: Arc<dyn Sandbox>,
    /// Base directory for the agent (for resolving relative paths).
    agent_dir: PathBuf,
    /// Cached README content by tool name.
    readme_cache: RwLock<HashMap<String, String>>,
    /// Tool policy for command filtering.
    policy: ToolPolicy,
    /// Notification configuration.
    notify_config: NotifyConfig,
    /// Session ID for notifications (optional).
    session_id: Option<String>,
    /// Agent name for notifications.
    agent_name: String,
}

impl ToolExecutor {
    /// Create a new tool executor.
    pub fn new(
        tools: Vec<ToolConfig>,
        sandbox: Arc<dyn Sandbox>,
        agent_dir: PathBuf,
        policy: ToolPolicy,
        agent_name: String,
    ) -> Self {
        let tools_map = tools
            .into_iter()
            .map(|tc| {
                let name = match &tc {
                    ToolConfig::Builtin { name } => name.clone(),
                    ToolConfig::Cli { name, .. } => name.clone(),
                };
                (name, tc)
            })
            .collect();

        let notify_config = policy.notify.clone();

        Self {
            tools: tools_map,
            sandbox,
            agent_dir,
            readme_cache: RwLock::new(HashMap::new()),
            policy,
            notify_config,
            session_id: None,
            agent_name,
        }
    }

    /// Set the session ID for notifications.
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Execute a tool call and return the result.
    ///
    /// Checks policy before execution:
    /// - If denied by policy, returns `PolicyDenied` error
    /// - If approval required (ask mode), returns `ApprovalRequired` error
    /// - If allowed, executes and optionally sends notifications
    pub async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, ToolError> {
        let tool_name = &tool_call.function.name;
        let config = self
            .tools
            .get(tool_name)
            .ok_or_else(|| ToolError::NotFound(tool_name.clone()))?;

        // Determine tool type and invocation string for policy check
        let (tool_type, invocation) = match config {
            ToolConfig::Builtin { name } if name == "bash" => {
                // For bash, extract the command from arguments
                let command = extract_bash_command(&tool_call.function.arguments);
                (ToolType::Bash, command)
            }
            ToolConfig::Builtin { name } => (ToolType::Builtin, name.clone()),
            ToolConfig::Cli { name, .. } => (ToolType::Builtin, name.clone()),
        };

        // Check policy
        match self.policy.check(tool_type, &invocation) {
            PolicyDecision::Deny => {
                return Err(ToolError::PolicyDenied(invocation));
            }
            PolicyDecision::Ask => {
                return Err(ToolError::ApprovalRequired {
                    call_id: tool_call.id.clone(),
                    command: invocation,
                });
            }
            PolicyDecision::Allow => {
                // Continue with execution
            }
        }

        self.execute_tool(tool_call, config, tool_type, &invocation)
            .await
    }

    /// Execute a tool call bypassing policy checks.
    ///
    /// Use this only for calls that have already been approved through the
    /// approval flow. Skips policy.check() but still executes the tool and
    /// sends notifications.
    pub async fn execute_bypassing_policy(
        &self,
        tool_call: &ToolCall,
    ) -> Result<ToolResult, ToolError> {
        let tool_name = &tool_call.function.name;
        let config = self
            .tools
            .get(tool_name)
            .ok_or_else(|| ToolError::NotFound(tool_name.clone()))?;

        // Determine tool type and invocation string (for notifications)
        let (tool_type, invocation) = match config {
            ToolConfig::Builtin { name } if name == "bash" => {
                let command = extract_bash_command(&tool_call.function.arguments);
                (ToolType::Bash, command)
            }
            ToolConfig::Builtin { name } => (ToolType::Builtin, name.clone()),
            ToolConfig::Cli { name, .. } => (ToolType::Builtin, name.clone()),
        };

        self.execute_tool(tool_call, config, tool_type, &invocation)
            .await
    }

    /// Internal: execute tool and send notifications.
    async fn execute_tool(
        &self,
        tool_call: &ToolCall,
        config: &ToolConfig,
        tool_type: ToolType,
        invocation: &str,
    ) -> Result<ToolResult, ToolError> {
        let result = match config {
            ToolConfig::Builtin { name } if name == "bash" => {
                bash::execute(
                    &self.sandbox,
                    &self.agent_dir,
                    &tool_call.function.arguments,
                )
                .await
            }
            ToolConfig::Builtin { name } => {
                Err(ToolError::NotFound(format!("unknown builtin: {}", name)))
            }
            ToolConfig::Cli { command, .. } => {
                cli::execute(
                    &self.sandbox,
                    &self.agent_dir,
                    command,
                    &tool_call.function.arguments,
                )
                .await
            }
        };

        // Send notification if configured
        if self.policy.should_notify(tool_type, invocation) {
            let session_id = self.session_id.as_deref().unwrap_or("unknown");
            let success = result.as_ref().map(|r| r.success).unwrap_or(false);
            send_notification(
                &self.notify_config,
                session_id,
                &self.agent_name,
                invocation,
                success,
            )
            .await;
        }

        result
    }

    /// Generate tool definitions for the LLM.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|config| match config {
                ToolConfig::Builtin { name } if name == "bash" => bash::definition(),
                ToolConfig::Builtin { name } => unknown_builtin_definition(name),
                ToolConfig::Cli {
                    name, description, ..
                } => cli::definition(name, description.as_deref()),
            })
            .collect()
    }

    /// Load and cache the README for a tool.
    pub async fn load_readme(&self, tool_name: &str) -> Option<String> {
        // Check cache first
        {
            let cache = self.readme_cache.read().await;
            if let Some(content) = cache.get(tool_name) {
                return Some(content.clone());
            }
        }

        // Get the tool config
        let config = self.tools.get(tool_name)?;
        let readme_path = match config {
            ToolConfig::Cli {
                readme: Some(r), ..
            } => self.agent_dir.join(r),
            _ => return None,
        };

        // Read the README file
        let content = fs::read_to_string(&readme_path).await.ok()?;

        // Cache it
        {
            let mut cache = self.readme_cache.write().await;
            cache.insert(tool_name.to_string(), content.clone());
        }

        Some(content)
    }

    /// Check if any tools are configured.
    pub fn has_tools(&self) -> bool {
        !self.tools.is_empty()
    }
}

// ============================================================================
// Private Helpers
// ============================================================================

/// Generate a fallback definition for an unknown builtin.
fn unknown_builtin_definition(name: &str) -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: name.to_string(),
            description: format!("Built-in tool: {}", name),
            parameters: None,
        },
    }
}

/// Extract the command string from bash tool arguments.
fn extract_bash_command(arguments: &str) -> String {
    #[derive(serde::Deserialize)]
    struct BashArgs {
        command: String,
    }

    serde_json::from_str::<BashArgs>(arguments)
        .map(|args| args.command)
        .unwrap_or_else(|_| arguments.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::TrustSandbox;
    use tempfile::TempDir;

    fn test_executor(tools: Vec<ToolConfig>) -> ToolExecutor {
        let sandbox = Arc::new(TrustSandbox);
        ToolExecutor::new(
            tools,
            sandbox,
            std::path::PathBuf::from("/tmp"),
            ToolPolicy::default(),
            "test-agent".to_string(),
        )
    }

    fn test_executor_with_dir(tools: Vec<ToolConfig>, dir: &TempDir) -> ToolExecutor {
        let sandbox = Arc::new(TrustSandbox);
        ToolExecutor::new(
            tools,
            sandbox,
            dir.path().to_path_buf(),
            ToolPolicy::default(),
            "test-agent".to_string(),
        )
    }

    #[test]
    fn tool_definitions_for_builtin_bash() {
        let executor = test_executor(vec![ToolConfig::Builtin {
            name: "bash".to_string(),
        }]);

        let defs = executor.tool_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].function.name, "bash");
        assert!(defs[0].function.parameters.is_some());
    }

    #[test]
    fn tool_definitions_for_cli() {
        let executor = test_executor(vec![ToolConfig::Cli {
            name: "git-helper".to_string(),
            command: "./tools/git-helper.sh".to_string(),
            readme: None,
            description: Some("Run git commands".to_string()),
        }]);

        let defs = executor.tool_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].function.name, "git-helper");
        assert_eq!(defs[0].function.description, "Run git commands");
    }

    #[tokio::test]
    async fn execute_bash_command() {
        let temp_dir = TempDir::new().unwrap();
        let executor = test_executor_with_dir(
            vec![ToolConfig::Builtin {
                name: "bash".to_string(),
            }],
            &temp_dir,
        );

        let tool_call = ToolCall {
            id: "call_1".to_string(),
            tool_type: "function".to_string(),
            function: crate::llm::FunctionCall {
                name: "bash".to_string(),
                arguments: r#"{"command": "echo hello"}"#.to_string(),
            },
        };

        let result = executor.execute(&tool_call).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("hello"));
    }

    #[test]
    fn has_tools_returns_false_when_empty() {
        let executor = test_executor(vec![]);
        assert!(!executor.has_tools());
    }

    #[test]
    fn has_tools_returns_true_when_configured() {
        let executor = test_executor(vec![ToolConfig::Builtin {
            name: "bash".to_string(),
        }]);
        assert!(executor.has_tools());
    }
}
