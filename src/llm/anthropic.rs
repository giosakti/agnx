//! Anthropic LLM provider with native API format.

use async_trait::async_trait;
use reqwest::Client;

use super::error::LLMError;
use super::provider::LLMProvider;
use super::types::{
    ChatRequest, ChatResponse, ChatStream, Choice, Message, Role, StreamEvent, Usage,
};

/// Anthropic provider with native API format.
pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    api_version: String,
}

impl AnthropicProvider {
    pub const DEFAULT_API_VERSION: &'static str = "2023-06-01";

    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            api_version: Self::DEFAULT_API_VERSION.to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let url = format!("{}/v1/messages", self.base_url);
        let anthropic_request = to_request(&request);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .json(&anthropic_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(LLMError::Api { status, message });
        }

        let anthropic_response: Response = response.json().await?;
        Ok(from_response(anthropic_response))
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, LLMError> {
        let url = format!("{}/v1/messages", self.base_url);
        let anthropic_request = to_stream_request(&request);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .json(&anthropic_request)
            .send()
            .await?;

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

// --- Request/Response types ---

#[derive(serde::Serialize)]
struct Request {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<RequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(serde::Serialize)]
struct StreamRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<RequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(serde::Serialize)]
struct RequestMessage {
    role: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct Response {
    id: String,
    content: Vec<Content>,
    stop_reason: Option<String>,
    usage: Option<ResponseUsage>,
}

#[derive(serde::Deserialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(serde::Deserialize)]
struct ResponseUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// --- Conversions ---

fn to_request(request: &ChatRequest) -> Request {
    let mut system = None;
    let mut messages = Vec::new();

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                system = Some(msg.content.clone());
            }
            Role::User => {
                messages.push(RequestMessage {
                    role: "user".to_string(),
                    content: msg.content.clone(),
                });
            }
            Role::Assistant => {
                messages.push(RequestMessage {
                    role: "assistant".to_string(),
                    content: msg.content.clone(),
                });
            }
        }
    }

    Request {
        model: request.model.clone(),
        max_tokens: request.max_tokens.unwrap_or(4096),
        system,
        messages,
        temperature: request.temperature,
    }
}

fn to_stream_request(request: &ChatRequest) -> StreamRequest {
    let base = to_request(request);
    StreamRequest {
        model: base.model,
        max_tokens: base.max_tokens,
        system: base.system,
        messages: base.messages,
        temperature: base.temperature,
        stream: true,
    }
}

fn from_response(response: Response) -> ChatResponse {
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

// --- Streaming ---

struct StreamParser<S> {
    inner: S,
    buffer: String,
    done: bool,
    usage: Option<Usage>,
}

impl<S> StreamParser<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
            done: false,
            usage: None,
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
            if let Some(line_end) = self.buffer.find('\n') {
                let line = self.buffer[..line_end].trim().to_string();
                self.buffer = self.buffer[line_end + 1..].to_string();

                if line.is_empty() || line.starts_with("event:") {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ")
                    && let Ok(event) = serde_json::from_str::<StreamEvent_>(data)
                {
                    match event {
                        StreamEvent_::ContentBlockDelta { delta } => {
                            if let Some(text) = delta.text
                                && !text.is_empty()
                            {
                                return Poll::Ready(Some(Ok(StreamEvent::Token(text))));
                            }
                        }
                        StreamEvent_::MessageDelta { usage: Some(u), .. } => {
                            self.usage = Some(Usage {
                                prompt_tokens: 0,
                                completion_tokens: u.output_tokens,
                                total_tokens: u.output_tokens,
                            });
                        }
                        StreamEvent_::MessageStop => {
                            self.done = true;
                            return Poll::Ready(Some(Ok(StreamEvent::Done {
                                usage: self.usage.take(),
                            })));
                        }
                        _ => {}
                    }
                }
                continue;
            }

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
                    return Poll::Ready(Some(Ok(StreamEvent::Done {
                        usage: self.usage.take(),
                    })));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)] // Fields needed for serde deserialization
enum StreamEvent_ {
    MessageStart {
        message: Option<serde_json::Value>,
    },
    ContentBlockStart {
        index: Option<u32>,
        content_block: Option<serde_json::Value>,
    },
    ContentBlockDelta {
        delta: Delta,
    },
    ContentBlockStop {
        index: Option<u32>,
    },
    MessageDelta {
        delta: Option<serde_json::Value>,
        usage: Option<StreamUsage>,
    },
    MessageStop,
    Ping,
    #[serde(other)]
    Unknown,
}

#[derive(serde::Deserialize)]
struct Delta {
    text: Option<String>,
}

#[derive(serde::Deserialize)]
struct StreamUsage {
    output_tokens: u32,
}
