# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands
- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Test single item: `cargo test <test_name>`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Architecture
- Tool type: CLI and MCP server for secret management.
- Language: Rust.
- Goals: Secure storage and retrieval of secrets.
