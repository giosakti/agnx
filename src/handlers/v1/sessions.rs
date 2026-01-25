use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::llm::{ChatRequest, Message, Role};
use crate::response;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    agent: String,
}

#[derive(Serialize)]
pub struct CreateSessionResponse {
    session_id: String,
    agent: String,
    status: String,
    created_at: String,
}

#[derive(Serialize)]
pub struct GetSessionResponse {
    session_id: String,
    agent: String,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    content: String,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    message_id: String,
    role: String,
    content: String,
}

/// POST /api/v1/sessions
pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Response {
    // Verify agent exists
    if state.agents.get(&req.agent).is_none() {
        return response::not_found(format!("Agent '{}' not found", req.agent)).into_response();
    }

    let session = state.sessions.create(req.agent).await;

    let response = CreateSessionResponse {
        session_id: session.id,
        agent: session.agent,
        status: session.status.to_string(),
        created_at: session.created_at.to_rfc3339(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// GET /api/v1/sessions/{session_id}
pub async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Response {
    let Some(session) = state.sessions.get(&session_id).await else {
        return response::not_found("Session not found").into_response();
    };

    let response = GetSessionResponse {
        session_id: session.id,
        agent: session.agent,
        status: session.status.to_string(),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// POST /api/v1/sessions/{session_id}/messages
pub async fn send_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    // Get session
    let Some(session) = state.sessions.get(&session_id).await else {
        return response::not_found("Session not found").into_response();
    };

    // Get agent spec
    let Some(agent) = state.agents.get(&session.agent) else {
        return response::internal_error("Session references non-existent agent").into_response();
    };

    // Add user message to session
    let user_message = Message {
        role: Role::User,
        content: req.content,
    };
    if state
        .sessions
        .add_message(&session_id, user_message.clone())
        .await
        .is_none()
    {
        return response::internal_error("Failed to add message to session").into_response();
    }

    // Build messages for LLM request
    let mut messages = Vec::new();

    // Build system message from system_prompt and instructions
    let mut system_content = String::new();
    if let Some(ref prompt) = agent.system_prompt {
        system_content.push_str(prompt);
    }
    if let Some(ref instructions) = agent.instructions {
        if !system_content.is_empty() {
            system_content.push_str("\n\n");
        }
        system_content.push_str(instructions);
    }
    if !system_content.is_empty() {
        messages.push(Message {
            role: Role::System,
            content: system_content,
        });
    }

    // Add conversation history
    if let Some(history) = state.sessions.get_messages(&session_id).await {
        messages.extend(history);
    }

    // Get provider from registry (with optional base_url from agent config)
    let Some(provider) = state
        .providers
        .get(&agent.model.provider, agent.model.base_url.as_deref())
    else {
        return response::internal_error(format!(
            "Provider '{}' not configured. Check API key environment variable.",
            agent.model.provider
        ))
        .into_response();
    };

    // Make LLM request
    let chat_request = ChatRequest {
        model: agent.model.name.clone(),
        messages,
        temperature: agent.model.temperature,
        max_tokens: agent.model.max_output_tokens,
    };

    let chat_response = match provider.chat(chat_request).await {
        Ok(resp) => resp,
        Err(e) => {
            return response::internal_error(format!("LLM request failed: {}", e)).into_response();
        }
    };

    // Extract assistant response
    let assistant_content = chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Add assistant message to session
    let assistant_message = Message {
        role: Role::Assistant,
        content: assistant_content.clone(),
    };
    let _ = state
        .sessions
        .add_message(&session_id, assistant_message)
        .await;

    let response = SendMessageResponse {
        message_id: format!("msg_{}", Uuid::new_v4().simple()),
        role: "assistant".to_string(),
        content: assistant_content,
    };

    (StatusCode::OK, Json(response)).into_response()
}
