//! OpenAI-compatible LLM provider.
//!
//! Works with OpenAI, OpenRouter, Ollama, and other compatible APIs.

use async_trait::async_trait;
use reqwest::Client;

use super::error::LLMError;
use super::provider::LLMProvider;
use super::types::{ChatRequest, ChatResponse, ChatStream, Message, StreamEvent, Usage};

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

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, LLMError> {
        let url = format!("{}/chat/completions", self.base_url);

        let stream_request = StreamRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: true,
        };

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.json(&stream_request).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(LLMError::Api { status, message });
        }

        let byte_stream = response.bytes_stream();
        let event_stream = StreamParser::new(byte_stream);

        Ok(Box::pin(event_stream))
    }
}

// --- Streaming types ---

#[derive(serde::Serialize)]
struct StreamRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

struct StreamParser<S> {
    inner: S,
    buffer: String,
    done: bool,
}

impl<S> StreamParser<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
            done: false,
        }
    }
}

impl<S> futures::Stream for StreamParser<S>
where
    S: futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<StreamEvent, LLMError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        if self.done {
            return Poll::Ready(None);
        }

        loop {
            // Try to parse a complete line from buffer
            if let Some(line_end) = self.buffer.find('\n') {
                let line = self.buffer[..line_end].trim().to_string();
                self.buffer = self.buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // Handle SSE data lines
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        self.done = true;
                        return Poll::Ready(Some(Ok(StreamEvent::Done { usage: None })));
                    }

                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(choice) = chunk.choices.first()
                            && let Some(ref content) = choice.delta.content
                            && !content.is_empty()
                        {
                            return Poll::Ready(Some(Ok(StreamEvent::Token(content.clone()))));
                        }
                        if chunk.usage.is_some() {
                            continue;
                        }
                    }
                }
                continue;
            }

            // Need more data
            match std::pin::Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LLMError::Request(e))));
                }
                Poll::Ready(None) => {
                    self.done = true;
                    if !self.buffer.is_empty() {
                        continue;
                    }
                    return Poll::Ready(Some(Ok(StreamEvent::Done { usage: None })));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(serde::Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(serde::Deserialize)]
struct StreamDelta {
    content: Option<String>,
}
