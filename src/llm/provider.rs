//! LLM provider trait and implementations.

use async_trait::async_trait;
use reqwest::Client;

use super::error::LLMError;
use super::types::{ChatRequest, ChatResponse, Message, Role};

/// Trait for LLM providers with different API formats.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Make a chat completion request.
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;
}

/// OpenAI-compatible provider (works for OpenAI, OpenRouter, Ollama).
pub struct OpenAICompatibleProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl OpenAICompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatibleProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(LLMError::Api { status, message });
        }

        Ok(response.json().await?)
    }
}

/// Anthropic provider with native API format.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl AnthropicProvider {
    const BASE_URL: &'static str = "https://api.anthropic.com";
    const API_VERSION: &'static str = "2023-06-01";

    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let url = format!("{}/v1/messages", Self::BASE_URL);

        // Transform to Anthropic format
        let anthropic_request = to_anthropic_request(&request);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", Self::API_VERSION)
            .json(&anthropic_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(LLMError::Api { status, message });
        }

        // Transform response back to common format
        let anthropic_response: AnthropicResponse = response.json().await?;
        Ok(from_anthropic_response(anthropic_response))
    }
}

// --- Anthropic format types and conversions ---

#[derive(serde::Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(serde::Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(serde::Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(serde::Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

fn to_anthropic_request(request: &ChatRequest) -> AnthropicRequest {
    let mut system = None;
    let mut messages = Vec::new();

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                // Anthropic wants system as a separate field
                system = Some(msg.content.clone());
            }
            Role::User => {
                messages.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: msg.content.clone(),
                });
            }
            Role::Assistant => {
                messages.push(AnthropicMessage {
                    role: "assistant".to_string(),
                    content: msg.content.clone(),
                });
            }
        }
    }

    AnthropicRequest {
        model: request.model.clone(),
        max_tokens: request.max_tokens.unwrap_or(4096),
        system,
        messages,
        temperature: request.temperature,
    }
}

fn from_anthropic_response(response: AnthropicResponse) -> ChatResponse {
    use super::types::{Choice, Usage};

    let content = response
        .content
        .into_iter()
        .filter(|c| c.content_type == "text")
        .map(|c| c.text)
        .collect::<Vec<_>>()
        .join("");

    ChatResponse {
        id: response.id,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
            },
            finish_reason: response.stop_reason,
        }],
        usage: response.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        }),
    }
}
