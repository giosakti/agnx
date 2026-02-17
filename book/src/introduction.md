# Introduction

**Duragent** — A durable, self-contained runtime for AI agents.

Sessions survive crashes. Agents are just files. One binary, zero dependencies.

Use it as a personal AI assistant, or as the foundation for agent-powered products.

## Why Duragent?

| What you get | How |
|--------------|-----|
| Sessions that survive crashes | Append-only event log, attach/detach like tmux |
| Agents you can read and version | YAML + Markdown — no code required |
| State you can inspect | Just files on disk — `cat`, `grep`, `git diff` |
| Deploy anywhere | Single binary, ~10MB, no Python/Node/Docker |
| Your choice of parts | Swap LLM providers, gateways, and storage backends or bring your own |

## Features

- **Durable sessions** — crash, restart, reconnect; your conversation survives
- **Portable agent format** — define agents in YAML + Markdown; inspect, version, and share them
- **Memory** — agents recall past conversations, remember experiences, and reflect on long-term knowledge
- **Tools** — bash execution, CLI tools, web search/fetch, scheduled tasks, and background processes, with configurable approval policies
- **Skills** — modular capabilities defined as Markdown files ([Agent Skills](https://agentskills.io) standard)
- **Context management** — token budgeting, history truncation, and priority-based context rendering
- **Multiple LLM providers** — Anthropic, OpenAI, OpenRouter, Ollama
- **Platform gateways** — Telegram and Discord via subprocess plugins; group chat with mention gating and debouncing
- **HTTP API** — REST endpoints with SSE streaming
- **Operational tooling** — `duragent doctor` for diagnostics, `duragent upgrade` for self-update

## Modular by Design

| Component | Default | Swappable |
|-----------|---------|-----------|
| Gateways | CLI, HTTP, SSE, Telegram, Discord | Any platform via gateway plugins |
| LLM | OpenRouter | Anthropic, OpenAI, Ollama, or any provider |
| Sandbox | Trust mode | bubblewrap, Docker *(planned)* |
| Storage | Filesystem | Postgres, S3 *(planned)* |
