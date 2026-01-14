# Agnx

> **Agnx is the "nginx for AI agents"** — a minimal, fast, self-hostable runtime that runs agents defined in a **transparent, portable format**, exposed through a standard API.

Agnx treats agents as durable artifacts: files you own that should outlast the runtime.
- **Transparent agent format** (human-readable, inspectable, versionable)
- **Stateless by default** (no hidden server-side state)
- **File-based state** when present (specs, memories, logs, config) — if Agnx disappears, take these and host elsewhere

## Documentation

- **[Project status / roadmap](./docs/PROJECT_STATUS.md)**
- **[Project Charter](./docs/plans/202601111100.project-charter.md)**
- **[Architecture](./docs/plans/202601111101.architecture.md)**
- **[API Reference](./docs/plans/202601111102.api-reference.md)**
- **[Deployment](./docs/plans/202601111103.deployment.md)**
- **[Agnx Agent Format (AAF)](./docs/plans/202601111200.agnx-agent-format.md)**
- **[Example skill](./docs/examples/skills/task-extraction/)**

## Tech Stack

- Go 1.25+ (single-binary, minimal dependencies)
- HTTP API: `net/http` + SSE
- Config/spec: YAML + Markdown
- Tool ecosystem: MCP
- Discovery: A2A Agent Card

## Workspace Layout (file-based mode)

Agnx's default workspace layout is:

```
./.agnx/
├── agents/<agent-name>/
│   ├── agent.yaml
│   ├── SYSTEM_PROMPT.md
│   ├── INSTRUCTIONS.md
│   └── skills/              # default local skill discovery
├── memory/                  # user-owned durable memory (files)
├── sessions/                # user-owned chat/task history (files)
└── artifacts/               # user-owned outputs (files)
```

## License

See [LICENSE](LICENSE).
