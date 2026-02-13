# Installation

## Requirements

- Rust 1.85+ (stable toolchain)
- Linux, macOS, or Windows

## From Source

```bash
git clone https://github.com/giosakti/duragent.git
cd duragent
make build
./target/release/duragent --version
```

## Cargo Install

```bash
cargo install --git https://github.com/giosakti/duragent.git
```

## Verify Installation

```bash
duragent --version
```

## Gateway Plugins

Gateway plugins (Telegram, Discord) are separate binaries. To install them:

```bash
# Discord gateway
cargo install --git https://github.com/giosakti/duragent.git duragent-gateway-discord

# Telegram gateway
cargo install --git https://github.com/giosakti/duragent.git duragent-gateway-telegram
```
