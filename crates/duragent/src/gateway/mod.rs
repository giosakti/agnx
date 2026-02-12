//! Gateway system for platform integrations (Telegram, WhatsApp, etc.).
//!
//! Gateways enable Duragent to communicate with messaging platforms. The system supports:
//!
//! - **Built-in gateways**: Compiled into Duragent, communicate via Rust channels
//! - **External gateways**: Subprocess plugins, communicate via JSON over stdio
//!
//! Both types implement the same Gateway Protocol, allowing uniform handling.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         Duragent Core                                │
//! │                                                                  │
//! │  ┌─────────────────────────────────────────────────────────────┐ │
//! │  │                    Gateway Manager                          │ │
//! │  │   Routes messages between sessions and gateways             │ │
//! │  └──────────────────────────┬──────────────────────────────────┘ │
//! │                             │                                    │
//! │    ┌────────────────────────┴────────────────────────────────┐   │
//! │    │ Built-in Gateways (feature flags)                       │   │
//! │    │  Communication: Rust mpsc channels                      │   │
//! │    └────────────────────────┬────────────────────────────────┘   │
//! │                             │                                    │
//! └─────────────────────────────┼────────────────────────────────────┘
//!                               │ JSON Lines over stdio
//!                     ┌─────────┴─────────┐
//!                     │ External Gateways │
//!                     │ (subprocess)      │
//!                     └───────────────────┘
//! ```
//!
//! # Protocol
//!
//! The Gateway Protocol defines two message types:
//!
//! - [`GatewayCommand`]: Messages from Duragent to gateway (send message, typing, etc.)
//! - [`GatewayEvent`]: Messages from gateway to Duragent (message received, errors, etc.)
//!
//! For external gateways, these are serialized as JSON Lines (newline-delimited JSON).
//!
//! # Message Flow
//!
//! When a message arrives from a platform, it flows through these stages:
//!
//! ```text
//!  Gateway (Telegram/Discord/...)
//!       │  GatewayEvent::MessageReceived
//!       ▼
//!  GatewayMessageHandler::handle_message()          [handler.rs]
//!       │  1. Resolve agent via routing rules        [routing.rs]
//!       │  2. Get or create session                  [routing.rs]
//!       │  3. Resolve queue config (DM vs group)
//!       ▼
//!  SessionMessageQueue::debounce_or_enqueue()       [queue.rs]
//!       │
//!       ├─ Session idle → ProcessNow
//!       │     └─ process_message() immediately
//!       │
//!       └─ Session busy → Debounced
//!             └─ Timer fires → flush_debounce()
//!                   └─ Combined message enqueued
//!                         └─ wake notification
//!
//!  After processing completes:
//!  SessionMessageQueue::drain()                     [queue.rs]
//!       ├─ Batch mode  → all pending as one message
//!       ├─ Sequential  → next single message
//!       └─ Drop mode   → discard, go idle
//! ```

mod approval;
mod commands;
pub mod handler;
pub mod manager;
pub mod queue;
mod routing;
pub mod subprocess;

// Re-export protocol types from the protocol crate
pub use duragent_gateway_protocol::{
    AuthMethod, GatewayCommand, GatewayEvent, MediaPayload, MessageContent, MessageReceivedData,
    RoutingContext, Sender, capabilities, error_codes,
};

pub use handler::{GatewayHandlerConfig, GatewayMessageHandler};
pub use manager::{
    GatewayHandle, GatewayManager, GatewaySender, MessageHandler, SendError,
    build_approval_keyboard,
};
pub use routing::RoutingConfig;
pub use subprocess::SubprocessGateway;

// Re-export Discord gateway from the discord crate
#[cfg(feature = "gateway-discord")]
pub use duragent_gateway_discord::{DiscordConfig, DiscordGateway};

// Re-export Telegram gateway from the telegram crate
#[cfg(feature = "gateway-telegram")]
pub use duragent_gateway_telegram::{TelegramConfig, TelegramGateway};
