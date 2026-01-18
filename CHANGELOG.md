# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure with HTTP API
- Health check endpoints (`/livez`, `/readyz`, `/version`)
- Agent listing endpoint (`/api/v1/agents`)
- Configuration loading from YAML
- RFC 7807 error responses
- CI pipeline with linting, testing, and build verification

### Changed
- Project renamed from Pluto to Agnx

## [0.0.1] - 2026-01-11

### Added
- Initial repository setup
- Project documentation (architecture, API reference, deployment guide)
- Agnx Agent Format (AAF) specification

[Unreleased]: https://github.com/giosakti/agnx/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/giosakti/agnx/releases/tag/v0.0.1
