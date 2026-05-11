# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands
- Build (Release): `cargo build --release`
- Install Binary: `cargo install --path .`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Architecture
- **Type**: Desktop CLI / MCP Server.
- **Backend**: SQLite (via SeaORM).
- **Security**: Argon2id KDF + AES-256-GCM encryption.
- **Storage**: OS-standard data directories via `directories` crate.

## Coding Standards
- **Errors**: Use `anyhow::Result` for application logic.
- **Encryption**: Secrets MUST be encrypted before saving to DB.
- **Memory**: Use `Zeroize` for sensitive keys/passwords.
