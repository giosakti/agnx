.PHONY: build test test-nocapture lint coverage clean run watch help

# Build variables
BINARY_NAME := duragent
BUILD_DIR := target
VERSION := $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "none")
DATE := $(shell date -u +%Y-%m-%dT%H:%M:%SZ)

## help: Show this help message
help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@sed -n 's/^##//p' $(MAKEFILE_LIST) | column -t -s ':' | sed 's/^/ /'

## build: Build the binary
build:
	cargo build --release

## test: Run tests
test:
	cargo test --features server

## test-nocapture: Run tests with output (don't capture stdout/stderr)
test-nocapture:
	cargo test --features server -- --nocapture

## lint: Run linter
lint:
	cargo clippy --features server -- -D warnings

## coverage: Run tests with coverage report
coverage:
	cargo tarpaulin --out Html
	@echo "Coverage report: tarpaulin-report.html"

## clean: Remove build artifacts
clean:
	cargo clean
	rm -f tarpaulin-report.html

## run: Build and run the server
run:
	cargo run -- serve

## fmt: Format code
fmt:
	cargo fmt

## watch: Run tests on file change (requires cargo-watch)
watch:
	cargo watch -x 'test --features server'

## check: Run all checks (lint + test)
check: lint test
