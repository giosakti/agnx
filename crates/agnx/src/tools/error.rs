//! Tool execution errors.

use thiserror::Error;

/// Errors that can occur during tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tool not found in configuration.
    #[error("tool not found: {0}")]
    NotFound(String),

    /// Tool execution failed.
    #[error("tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Failed to parse tool arguments.
    #[error("failed to parse tool arguments: {0}")]
    InvalidArguments(String),

    /// Sandbox execution error.
    #[error("sandbox error: {0}")]
    Sandbox(#[from] crate::sandbox::SandboxError),

    /// I/O error (e.g., reading README).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
