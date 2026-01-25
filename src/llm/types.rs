//! Common types for LLM chat completions.

use serde::{Deserialize, Serialize};

/// A chat completion request (OpenAI-compatible format).
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// The role of a message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A chat completion response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

/// A single completion choice.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "openai/gpt-4".to_string(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: "You are a helpful assistant.".to_string(),
                },
                Message {
                    role: Role::User,
                    content: "Hello!".to_string(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"openai/gpt-4\""));
        assert!(json.contains("\"role\":\"system\""));
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn test_chat_request_without_optional_fields() {
        let request = ChatRequest {
            model: "openai/gpt-4".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: "Hi".to_string(),
            }],
            temperature: None,
            max_tokens: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].index, 0);
        assert_eq!(response.choices[0].message.role, Role::Assistant);
        assert_eq!(
            response.choices[0].message.content,
            "Hello! How can I help you today?"
        );
        assert_eq!(response.choices[0].finish_reason, Some("stop".to_string()));

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 8);
        assert_eq!(usage.total_tokens, 18);
    }

    #[test]
    fn test_chat_response_without_usage() {
        let json = r#"{
            "id": "chatcmpl-456",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": null
                }
            ]
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "chatcmpl-456");
        assert!(response.usage.is_none());
        assert!(response.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_message_roles() {
        let system = Role::System;
        let user = Role::User;
        let assistant = Role::Assistant;

        assert_eq!(serde_json::to_string(&system).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&user).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&assistant).unwrap(), "\"assistant\"");

        assert_eq!(
            serde_json::from_str::<Role>("\"system\"").unwrap(),
            Role::System
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"user\"").unwrap(),
            Role::User
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"assistant\"").unwrap(),
            Role::Assistant
        );
    }
}
