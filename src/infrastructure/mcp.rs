use crate::application::use_cases::SecretService;
use crate::domain::repositories::SecretRepository;
use async_trait::async_trait;
use mcp_sdk_rs::{
    error::{Error, ErrorCode},
    server::ServerHandler,
    transport::stdio::StdioTransport,
    types::{Implementation, ServerCapabilities},
};
use std::sync::Arc;

pub struct SecretManagerHandler<R: SecretRepository> {
    service: Arc<SecretService<R>>,
}

impl<R: SecretRepository> SecretManagerHandler<R> {
    pub fn new(service: SecretService<R>) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

#[async_trait]
impl<R: SecretRepository + 'static> ServerHandler for SecretManagerHandler<R> {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: mcp_sdk_rs::types::ClientCapabilities,
    ) -> Result<ServerCapabilities, Error> {
        Ok(ServerCapabilities {
            tools: Some(serde_json::json!({})),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        let params = params.unwrap_or(serde_json::json!({}));

        match method {
            "tools/list" => {
                Ok(serde_json::json!({
                    "tools": [
                        {
                            "name": "add_secret",
                            "description": "Add a new secret",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "value": { "type": "string" }
                                },
                                "required": ["name", "value"]
                            }
                        },
                        {
                            "name": "get_secret",
                            "description": "Retrieve a secret name",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" }
                                },
                                "required": ["name"]
                            }
                        },
                        {
                            "name": "list_secrets",
                            "description": "List all stored secret names",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                }))
            }
            "tools/call" => {
                let tool_name = params["name"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "missing tool name"))?;
                let arguments = &params["arguments"];

                match tool_name {
                    "add_secret" => {
                        let name = arguments["name"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "missing name"))?;
                        let value = arguments["value"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "missing value"))?;

                        self.service.add_secret(name.to_string(), value).await.map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        Ok(serde_json::json!({ "content": [{ "type": "text", "text": format!("Secret '{}' saved", name) }] }))
                    }
                    "get_secret" => {
                        let name = arguments["name"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "missing name"))?;
                        match self.service.get_secret(name).await.map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))? {
                            Some(v) => Ok(serde_json::json!({ "content": [{ "type": "text", "text": format!("{}: {}", name, v) }] })),
                            None => Ok(serde_json::json!({ "content": [{ "type": "text", "text": "not found" }] })),
                        }
                    }
                    "list_secrets" => {
                        let secrets = self.service.list_secrets().await.map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        let text = if secrets.is_empty() {
                            "no secrets found".to_string()
                        } else {
                            secrets.join("\n")
                        };

                        Ok(serde_json::json!({ "content": [{ "type": "text", "text": text }] }))
                    }
                    _ => Err(Error::protocol(ErrorCode::MethodNotFound, format!("unknown tool: {}", tool_name))),
                }
            }
            _ => Err(Error::protocol(ErrorCode::MethodNotFound, format!("unknown method: {}", method))),
        }
    }
}

pub async fn run_mcp_server<R: SecretRepository + 'static>(
    service: SecretService<R>,
) -> anyhow::Result<()> {
    let handler = SecretManagerHandler::new(service);

    let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel::<String>(100);
    let (stdout_tx, stdout_rx) = tokio::sync::mpsc::channel::<String>(100);

    let transport = StdioTransport::new(stdin_rx, stdout_tx);

    let server = mcp_sdk_rs::server::Server::new(
        Arc::new(transport),
        Arc::new(handler),
    );

    // Read stdin in spawned task
    let stdin_tx_clone = stdin_tx.clone();
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut buf = String::new();
        loop {
            buf.clear();
            match tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = buf.trim().to_string();
                    if !line.is_empty() && stdin_tx_clone.send(line).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Write stdout in spawned task
    tokio::spawn(async move {
        let stdout = tokio::io::stdout();
        let mut stdout = tokio::io::BufWriter::new(stdout);
        let mut rx = stdout_rx;
        while let Some(msg) = rx.recv().await {
            use tokio::io::AsyncWriteExt;
            if stdout.write_all(format!("{}\n", msg).as_bytes()).await.is_err() {
                break;
            }
            if stdout.flush().await.is_err() {
                break;
            }
        }
    });

    server.start().await.map_err(|e| anyhow::anyhow!("MCP error: {:?}", e))
}
