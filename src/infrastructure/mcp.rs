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
                            "description": "Retrieve a secret by name",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" }
                                },
                                "required": ["name"]
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
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::io::AsyncWriteExt;
    use tokio::sync::broadcast;

    let handler = SecretManagerHandler::new(service);

    let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel::<String>(100);
    let (stdout_tx, mut stdout_rx) = tokio::sync::mpsc::channel::<String>(100);

    let transport = StdioTransport::new(stdin_rx, stdout_tx);

    let server = mcp_sdk_rs::server::Server::new(
        Arc::new(transport),
        Arc::new(handler),
    );

    let stdin = BufReader::new(tokio::io::stdin());

    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let shutdown_rx = shutdown_tx.subscribe();

    let stdin_handle = tokio::spawn(async move {
        let mut lines = stdin.lines();
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => break,
                result = lines.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            if stdin_tx.send(line).await.is_err() {
                                break;
                            }
                        }
                        Ok(None) | Err(_) => break,
                    }
                }
            }
        }
    });

    let stdout_handle = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => break,
                msg = stdout_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            let _ = stdout.write_all((msg + "\n").as_bytes()).await;
                            let _ = stdout.flush().await;
                        }
                        None => break,
                    }
                }
            }
        }
    });

    let result = server.start().await.map_err(|e| anyhow::anyhow!("MCP error: {:?}", e));

    let _ = shutdown_tx.send(());

    let _ = stdin_handle.await;
    let _ = stdout_handle.await;

    result
}
