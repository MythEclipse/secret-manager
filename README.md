# Secret Manager (Rust + MCP)

Secure local secret manager with CLI and MCP server support.

## Features
- **Local Encryption**: AES-256-GCM with Argon2id key derivation.
- **SQLite Storage**: Persistent local database in standard user data directories.
- **MCP Server**: Integration with AI assistants (Claude, Cursor, etc.).
- **Zero Configuration**: Automatic database and directory initialization.

## Installation

### From Source
```bash
git clone <repo-url>
cd secret-manager
cargo install --path .
```
This installs the `secret-manager` binary to your `~/.cargo/bin` directory, making it available system-wide.

## Usage

Set your master password via `SM_PASSWORD` environment variable for automation, or enter it when prompted in CLI mode.

### CLI Commands
```bash
# Add a secret
secret-manager add <name> <value>

# Get a secret
secret-manager get <name>

# List secrets
secret-manager list
```

### MCP Mode (Production Setup)
To use this with Claude Code or other MCP clients:

```bash
# Example for Claude Code
export SM_PASSWORD="your-master-password"
secret-manager mcp
```

## Desktop Integration (Best Practice)
The application follows OS standards for data storage:
- **Linux**: `~/.local/share/secret-manager`
- **macOS**: `~/Library/Application Support/com.antigravity.secret-manager`
- **Windows**: `C:\Users\<User>\AppData\Roaming\antigravity\secret-manager`

## Skill Integration (Claude Code)
Install the binary first using `cargo install --path .`, then add the skill:

```bash
/skill secret-manager "secret-manager mcp"
```

## Security
- **Argon2id**: Industry-standard key derivation function.
- **AES-256-GCM**: Authenticated encryption to prevent tampering.
- **Memory Safety**: Written in Rust to prevent memory-related vulnerabilities.
- **Zeroize**: Sensitive data is wiped from memory when no longer needed.
