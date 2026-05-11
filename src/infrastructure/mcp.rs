use crate::application::use_cases::SecretService;
use crate::domain::repositories::SecretRepository;
use async_trait::async_trait;
use futures::Stream;
use mcp_sdk_rs::{
    error::{Error, ErrorCode},
    server::ServerHandler,
    transport::Transport,
    types::{Implementation, ServerCapabilities},
};
use std::pin::Pin;
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

struct ChannelTransport {
    input: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<String>>>,
    output: tokio::sync::mpsc::Sender<String>,
}

impl ChannelTransport {
    fn new(input: tokio::sync::mpsc::Receiver<String>, output: tokio::sync::mpsc::Sender<String>) -> Self {
        Self {
            input: Arc::new(tokio::sync::Mutex::new(input)),
            output
        }
    }
}

#[async_trait]
impl Transport for ChannelTransport {
    async fn send(&self, message: mcp_sdk_rs::transport::Message) -> Result<(), Error> {
        let json = serde_json::to_string(&message).map_err(|e| Error::Transport(e.to_string()))?;
        self.output.send(json).await.map_err(|_| Error::Transport("send failed".to_string()))?;
        Ok(())
    }

    fn receive(&self) -> Pin<Box<dyn Stream<Item = Result<mcp_sdk_rs::transport::Message, Error>> + Send>> {
        let input = self.input.clone();
        Box::pin(async_stream::stream! {
            let mut input = input.lock().await;
            while let Some(line) = input.recv().await {
                match serde_json::from_str::<mcp_sdk_rs::transport::Message>(&line) {
                    Ok(msg) => yield Ok(msg),
                    Err(e) => yield Err(Error::Transport(e.to_string())),
                }
            }
        })
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

pub async fn run_mcp_server<R: SecretRepository + 'static>(
    service: SecretService<R>,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};

    let handler = SecretManagerHandler::new(service);

    let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel::<String>(100);
    let (stdout_tx, stdout_rx) = tokio::sync::mpsc::channel::<String>(100);

    let transport = ChannelTransport::new(stdin_rx, stdout_tx);

    let server = mcp_sdk_rs::server::Server::new(
        Arc::new(transport),
        Arc::new(handler),
    );

    let stdin = BufReader::new(tokio::io::stdin());

    tokio::spawn(async move {
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stdin_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        let mut rx = stdout_rx;
        while let Some(msg) = rx.recv().await {
            if stdout.write_all((msg + "\n").as_bytes()).await.is_err() {
                break;
            }
            if stdout.flush().await.is_err() {
                break;
            }
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    server.start().await.map_err(|e| anyhow::anyhow!("MCP error: {:?}", e))
}
