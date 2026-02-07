//! LLM provider client for chat completions.

mod error;
mod types;

#[cfg(feature = "server")]
mod anthropic;
#[cfg(feature = "server")]
mod openai;
#[cfg(feature = "server")]
mod provider;
#[cfg(feature = "server")]
mod registry;

pub use error::LLMError;
pub use types::{
    ChatRequest, ChatStream, FunctionCall, FunctionDefinition, Message, Role, StreamEvent,
    ToolCall, ToolDefinition, Usage,
};

#[cfg(feature = "server")]
pub use anthropic::{AnthropicAuth, AnthropicProvider};
#[cfg(feature = "server")]
pub use openai::OpenAICompatibleProvider;
#[cfg(feature = "server")]
pub use provider::{LLMProvider, Provider};
#[cfg(feature = "server")]
pub use registry::ProviderRegistry;
