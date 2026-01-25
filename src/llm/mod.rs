//! LLM provider client for chat completions.

mod error;
mod provider;
mod registry;
mod types;

pub use registry::ProviderRegistry;
pub use types::{ChatRequest, Message, Role};
