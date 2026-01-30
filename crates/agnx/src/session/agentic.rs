//! Agentic loop for tool-using agents.
//!
//! This module implements the core agentic loop:
//! 1. Build ChatRequest with tools
//! 2. Call LLM
//! 3. If response has tool_calls: execute tools, add results, continue
//! 4. If no tool_calls: return final message
//! 5. Check iteration limit

use std::path::Path;
use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, warn};

use crate::agent::AgentSpec;
use crate::llm::{ChatRequest, LLMProvider, Message, Role, StreamEvent, ToolCall, Usage};
use crate::session::{SessionEventPayload, SessionStore, record_event};
use crate::tools::{ToolError, ToolExecutor};

/// Result of running the agentic loop.
#[derive(Debug)]
pub struct AgenticResult {
    /// Final assistant response content.
    pub content: String,
    /// Total token usage across all iterations.
    pub usage: Option<Usage>,
    /// Number of iterations executed.
    pub iterations: u32,
    /// Tool calls made during the loop.
    pub tool_calls_made: u32,
}

/// Error from the agentic loop.
#[derive(Debug, thiserror::Error)]
pub enum AgenticError {
    #[error("llm error: {0}")]
    Llm(#[from] crate::llm::LLMError),

    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("max iterations ({0}) exceeded")]
    MaxIterationsExceeded(u32),
}

/// Run the agentic loop with tool execution.
///
/// This function implements the core loop for tool-using agents:
/// - Calls the LLM with available tools
/// - If the LLM returns tool calls, executes them and feeds results back
/// - Continues until the LLM returns a final response or max iterations is reached
pub async fn run_agentic_loop(
    provider: Arc<dyn LLMProvider>,
    executor: &ToolExecutor,
    agent_spec: &AgentSpec,
    initial_messages: Vec<Message>,
    sessions: &SessionStore,
    sessions_path: &Path,
    session_id: &str,
) -> Result<AgenticResult, AgenticError> {
    let max_iterations = agent_spec.session.max_tool_iterations;
    let tool_definitions = executor.tool_definitions();

    let mut messages = initial_messages;
    let mut total_usage: Option<Usage> = None;
    let mut iterations = 0u32;
    let mut tool_calls_made = 0u32;

    loop {
        iterations += 1;

        if iterations > max_iterations {
            return Err(AgenticError::MaxIterationsExceeded(max_iterations));
        }

        debug!(
            iteration = iterations,
            max_iterations,
            messages_count = messages.len(),
            "Agentic loop iteration"
        );

        // Build request with tools
        let request = ChatRequest::with_tools(
            &agent_spec.model.name,
            messages.clone(),
            agent_spec.model.temperature,
            agent_spec.model.max_output_tokens,
            tool_definitions.clone(),
        );

        // Call LLM with streaming
        let mut stream = provider.chat_stream(request).await?;

        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut usage: Option<Usage> = None;

        // Consume stream
        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::Token(token) => {
                    content.push_str(&token);
                }
                StreamEvent::ToolCalls(calls) => {
                    tool_calls = calls;
                }
                StreamEvent::Done { usage: u } => {
                    usage = u;
                }
                StreamEvent::Cancelled => {
                    break;
                }
            }
        }

        // Accumulate usage
        if let Some(u) = usage {
            total_usage = Some(match total_usage {
                Some(existing) => Usage {
                    prompt_tokens: existing.prompt_tokens + u.prompt_tokens,
                    completion_tokens: existing.completion_tokens + u.completion_tokens,
                    total_tokens: existing.total_tokens + u.total_tokens,
                },
                None => u,
            });
        }

        // If no tool calls, we're done
        if tool_calls.is_empty() {
            return Ok(AgenticResult {
                content,
                usage: total_usage,
                iterations,
                tool_calls_made,
            });
        }

        // Process tool calls
        debug!(tool_calls_count = tool_calls.len(), "Processing tool calls");

        // Add assistant message with tool calls
        let assistant_msg = if content.is_empty() {
            Message::assistant_tool_calls(tool_calls.clone())
        } else {
            Message {
                role: Role::Assistant,
                content: Some(content.clone()),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
            }
        };
        messages.push(assistant_msg);

        // Execute all tools in parallel
        let tool_futures: Vec<_> = tool_calls
            .iter()
            .map(|tool_call| executor.execute(tool_call))
            .collect();

        let results = futures::future::join_all(tool_futures).await;

        // Process results: record events and add messages
        for (tool_call, result) in tool_calls.iter().zip(results) {
            tool_calls_made += 1;

            // Record tool call event
            let arguments: serde_json::Value =
                serde_json::from_str(&tool_call.function.arguments).unwrap_or_default();
            if let Err(e) = record_event(
                sessions,
                sessions_path,
                session_id,
                SessionEventPayload::ToolCall {
                    call_id: tool_call.id.clone(),
                    tool_name: tool_call.function.name.clone(),
                    arguments: arguments.clone(),
                },
            )
            .await
            {
                warn!(error = %e, "Failed to record tool call event");
            }

            // Convert Result to ToolResult (handle errors gracefully)
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    // Feed error back to LLM
                    crate::tools::ToolResult {
                        success: false,
                        content: format!("Tool execution failed: {}", e),
                    }
                }
            };

            // Record tool result event
            if let Err(e) = record_event(
                sessions,
                sessions_path,
                session_id,
                SessionEventPayload::ToolResult {
                    call_id: tool_call.id.clone(),
                    result: crate::session::ToolResultData {
                        success: result.success,
                        content: result.content.clone(),
                    },
                },
            )
            .await
            {
                warn!(error = %e, "Failed to record tool result event");
            }

            // Add tool result message
            let tool_result_msg = Message::tool_result(&tool_call.id, result.content);
            messages.push(tool_result_msg);
        }

        // Continue loop with updated messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agentic_result_debug() {
        let result = AgenticResult {
            content: "Hello".to_string(),
            usage: None,
            iterations: 1,
            tool_calls_made: 0,
        };
        assert!(format!("{:?}", result).contains("Hello"));
    }
}
