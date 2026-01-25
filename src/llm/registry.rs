//! Provider registry for managing LLM provider instances.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{info, warn};

use super::provider::{AnthropicProvider, LLMProvider, OpenAICompatibleProvider};
use crate::agent::Provider;

/// Registry of LLM providers, keyed by provider type.
#[derive(Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<Provider, Arc<dyn LLMProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize providers from environment variables.
    pub fn from_env() -> Self {
        let mut registry = Self::new();

        // OpenRouter
        if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
            let provider = OpenAICompatibleProvider::new(
                "https://openrouter.ai/api/v1".to_string(),
                Some(api_key),
            );
            registry.register(Provider::OpenRouter, Arc::new(provider));
            info!("Registered OpenRouter provider");
        }

        // OpenAI
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            let provider = OpenAICompatibleProvider::new(
                "https://api.openai.com/v1".to_string(),
                Some(api_key),
            );
            registry.register(Provider::OpenAI, Arc::new(provider));
            info!("Registered OpenAI provider");
        }

        // Anthropic
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            let provider = AnthropicProvider::new(api_key);
            registry.register(Provider::Anthropic, Arc::new(provider));
            info!("Registered Anthropic provider");
        }

        // Ollama (no auth required, always available if running locally)
        let ollama = OpenAICompatibleProvider::new("http://localhost:11434/api".to_string(), None);
        registry.register(Provider::Ollama, Arc::new(ollama));
        info!("Registered Ollama provider");

        if registry.get(&Provider::OpenRouter).is_none()
            && registry.get(&Provider::OpenAI).is_none()
            && registry.get(&Provider::Anthropic).is_none()
        {
            warn!(
                "No cloud LLM providers configured. \
                Set OPENROUTER_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY."
            );
        }

        registry
    }

    /// Register a provider implementation.
    pub fn register(&mut self, provider: Provider, implementation: Arc<dyn LLMProvider>) {
        self.providers.insert(provider, implementation);
    }

    /// Get a provider by type.
    pub fn get(&self, provider: &Provider) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(provider).cloned()
    }
}
