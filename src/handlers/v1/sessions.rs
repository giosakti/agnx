//! Session management HTTP handlers.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::llm::{ChatRequest, LLMProvider, Message, Role, StreamEvent};
use crate::response;
use crate::server::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

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

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/v1/sessions
pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Response {
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
    let (chat_request, provider) =
        match prepare_chat_context(&state, &session_id, req.content).await {
            Ok(ctx) => ctx,
            Err(resp) => return resp,
        };

    let chat_response = match provider.chat(chat_request).await {
        Ok(resp) => resp,
        Err(e) => {
            return response::internal_error(format!("LLM request failed: {}", e)).into_response();
        }
    };

    let assistant_content = chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

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

/// POST /api/v1/sessions/{session_id}/stream
///
/// SSE endpoint for streaming chat completions.
///
/// Request body: `{"content": "..."}`
///
/// Events emitted:
/// - `start`: `{}` — signals streaming has begun
/// - `token`: `{"content": "..."}` — streamed content chunks
/// - `done`: `{"message_id": "msg_...", "usage": {...}}` — stream complete with message ID
/// - `error`: `{"message": "..."}` — on error (timeout, LLM failure)
pub async fn stream_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    let (chat_request, provider) =
        match prepare_chat_context(&state, &session_id, req.content).await {
            Ok(ctx) => ctx,
            Err(resp) => return resp,
        };

    let stream = match provider.chat_stream(chat_request).await {
        Ok(s) => s,
        Err(e) => {
            return response::internal_error(format!("LLM request failed: {}", e)).into_response();
        }
    };

    let message_id = format!("msg_{}", Uuid::new_v4().simple());

    let sse_stream = AccumulatingStream::new(
        stream,
        state.sessions.clone(),
        session_id,
        message_id,
        Duration::from_secs(state.idle_timeout_seconds),
    );

    let keep_alive = KeepAlive::new()
        .interval(Duration::from_secs(state.keep_alive_interval_seconds))
        .text("keep-alive");

    Sse::new(sse_stream).keep_alive(keep_alive).into_response()
}

// ============================================================================
// Helpers
// ============================================================================

/// Prepare chat context for LLM request.
///
/// Validates session and agent, adds user message, builds system prompt and history,
/// and returns the ChatRequest with the provider.
async fn prepare_chat_context(
    state: &AppState,
    session_id: &str,
    user_content: String,
) -> Result<(ChatRequest, Arc<dyn LLMProvider>), Response> {
    let Some(session) = state.sessions.get(session_id).await else {
        return Err(response::not_found("Session not found").into_response());
    };

    let Some(agent) = state.agents.get(&session.agent) else {
        return Err(
            response::internal_error("Session references non-existent agent").into_response(),
        );
    };

    let user_message = Message {
        role: Role::User,
        content: user_content,
    };
    if state
        .sessions
        .add_message(session_id, user_message)
        .await
        .is_none()
    {
        return Err(response::internal_error("Failed to add message to session").into_response());
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

    if let Some(history) = state.sessions.get_messages(session_id).await {
        messages.extend(history);
    }

    let Some(provider) = state
        .providers
        .get(&agent.model.provider, agent.model.base_url.as_deref())
    else {
        return Err(response::internal_error(format!(
            "Provider '{}' not configured. Check API key environment variable.",
            agent.model.provider
        ))
        .into_response());
    };

    let chat_request = ChatRequest {
        model: agent.model.name.clone(),
        messages,
        temperature: agent.model.temperature,
        max_tokens: agent.model.max_output_tokens,
    };

    Ok((chat_request, provider))
}

// ============================================================================
// SSE Streaming
// ============================================================================

// --- SSE Event Data Types ---

#[derive(Serialize)]
struct TokenData {
    content: String,
}

#[derive(Serialize)]
struct DoneData {
    message_id: String,
    usage: Option<crate::llm::Usage>,
}

#[derive(Serialize)]
struct ErrorData {
    message: String,
}

// --- Stream Types ---

/// Unified error type for streaming, flattening nested Results.
enum StreamError {
    Llm(crate::llm::LLMError),
    Timeout,
}

/// Inner stream type that flattens `Result<Result<T, LLMError>, Elapsed>` into `Result<T, StreamError>`.
type FlattenedLLMStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, StreamError>> + Send>>;

// --- AccumulatingStream ---

/// A stream wrapper that accumulates token content and stores the assistant message when done.
///
/// Features:
/// - Idle timeout via `tokio_stream::StreamExt::timeout()`
/// - Drop safety: saves partial messages if the connection aborts
/// - Emits `start` event before streaming, `done` event with message ID when complete
struct AccumulatingStream {
    inner: FlattenedLLMStream,
    message_id: String,
    accumulated: String,
    sessions: crate::session::SessionStore,
    session_id: String,
    started: bool,
    finished: bool,
}

impl AccumulatingStream {
    fn new(
        inner: crate::llm::ChatStream,
        sessions: crate::session::SessionStore,
        session_id: String,
        message_id: String,
        idle_timeout: Duration,
    ) -> Self {
        // Wrap the inner stream with timeout and flatten the nested Results
        let timed_stream = inner.timeout(idle_timeout);
        let flattened = tokio_stream::StreamExt::map(timed_stream, |result| match result {
            Ok(Ok(event)) => Ok(event),
            Ok(Err(llm_err)) => Err(StreamError::Llm(llm_err)),
            Err(_elapsed) => Err(StreamError::Timeout),
        });

        Self {
            inner: Box::pin(flattened),
            message_id,
            accumulated: String::new(),
            sessions,
            session_id,
            started: false,
            finished: false,
        }
    }

    /// Save accumulated content as assistant message.
    fn save_accumulated(&mut self) {
        if !self.accumulated.is_empty() {
            let sessions = self.sessions.clone();
            let session_id = self.session_id.clone();
            let content = std::mem::take(&mut self.accumulated);

            tokio::spawn(async move {
                let assistant_message = Message {
                    role: Role::Assistant,
                    content,
                };
                let _ = sessions.add_message(&session_id, assistant_message).await;
            });
        }
    }
}

impl Drop for AccumulatingStream {
    fn drop(&mut self) {
        // If stream wasn't finished normally but has accumulated content,
        // the connection likely dropped. Save what we have.
        if !self.finished && !self.accumulated.is_empty() {
            self.save_accumulated();
        }
    }
}

impl futures::Stream for AccumulatingStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        if self.finished {
            return Poll::Ready(None);
        }

        // Emit start event on first poll
        if !self.started {
            self.started = true;
            let event = Event::default().event("start").data("{}");
            return Poll::Ready(Some(Ok(event)));
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(StreamEvent::Token(content)))) => {
                self.accumulated.push_str(&content);
                let event = Event::default()
                    .event("token")
                    .json_data(TokenData { content })
                    .unwrap_or_else(|_| Event::default().event("token").data("{}"));
                Poll::Ready(Some(Ok(event)))
            }

            Poll::Ready(Some(Ok(StreamEvent::Done { usage }))) => {
                self.finished = true;
                self.save_accumulated();
                let event = Event::default()
                    .event("done")
                    .json_data(DoneData {
                        message_id: self.message_id.clone(),
                        usage,
                    })
                    .unwrap_or_else(|_| Event::default().event("done").data("{}"));
                Poll::Ready(Some(Ok(event)))
            }

            Poll::Ready(Some(Err(StreamError::Timeout))) => {
                self.finished = true;
                self.save_accumulated();
                let event = Event::default()
                    .event("error")
                    .json_data(ErrorData {
                        message: "Stream idle timeout".to_string(),
                    })
                    .unwrap_or_else(|_| Event::default().event("error").data("{}"));
                Poll::Ready(Some(Ok(event)))
            }

            Poll::Ready(Some(Err(StreamError::Llm(e)))) => {
                self.finished = true;
                self.save_accumulated();
                let event = Event::default()
                    .event("error")
                    .json_data(ErrorData {
                        message: e.to_string(),
                    })
                    .unwrap_or_else(|_| Event::default().event("error").data("{}"));
                Poll::Ready(Some(Ok(event)))
            }

            Poll::Ready(None) => {
                self.finished = true;
                self.save_accumulated();
                Poll::Ready(None)
            }

            Poll::Pending => Poll::Pending,
        }
    }
}
