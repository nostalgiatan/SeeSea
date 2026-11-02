//! A Model Context Protocol server over JSON-RPC 2.0.
//!
//! [`McpServer::handle`] is the whole protocol as a function from message to optional reply, so the
//! stdio loop and the HTTP endpoint share it and tests need no I/O. Three tools are offered:
//! `search`, `extract` and `sources`. A failing tool reports `isError: true` in its result, as the
//! specification asks; JSON-RPC errors are reserved for malformed requests.

use seesea_extract::Scope;
use seesea_pipeline::render::{extract_markdown, search_markdown};
use seesea_pipeline::{Error, Pipeline, SearchRequest};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Protocol versions this server can speak, newest first.
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

pub struct McpServer {
    pipeline: Arc<Pipeline>,
}

fn error_reply(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

fn ok_reply(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn tool_result(text: String, structured: Value, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "structuredContent": structured, "isError": is_error})
}

fn tool_error(e: &Error) -> Value {
    let body = e.body();
    tool_result(
        format!("{}: {}", body.code, body.message),
        serde_json::to_value(&body).unwrap_or(Value::Null),
        true,
    )
}

/// The tool definitions, with JSON Schema for the arguments.
pub fn tools() -> Value {
    json!([
        {
            "name": "search",
            "description": "Web search that fuses several engines, then fetches the best results and returns the parts of each page that answer the query. Use mode 'fast' for snippets only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "What to search for."},
                    "mode": {"type": "string", "enum": ["fast", "standard", "deep"], "default": "standard", "description": "fast: snippets only. standard: fetch and read the top results. deep: read more of them."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 10},
                    "time_range": {"type": "string", "enum": ["day", "week", "month", "year"]},
                    "language": {"type": "string", "description": "Language code such as 'en' or 'zh'."},
                    "sources": {"type": "array", "items": {"type": "string"}, "description": "Restrict to these sources; see the 'sources' tool."},
                    "max_chars_per_result": {"type": "integer", "minimum": 1, "default": 3000, "description": "Cap on page text returned per result."},
                    "full_content": {"type": "boolean", "default": false, "description": "Return whole pages instead of the relevant sections."}
                },
                "required": ["query"]
            }
        },
        {
            "name": "extract",
            "description": "Fetch one web page and return its main content as Markdown, without navigation, ads or footers. With a query, only the sections relevant to it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "http or https address of a public page."},
                    "query": {"type": "string", "description": "If given, return only the sections that answer this."},
                    "scope": {"type": "string", "enum": ["core", "extended"], "default": "core", "description": "extended also keeps comments and similar adjacent material."},
                    "max_chars": {"type": "integer", "minimum": 1, "default": 6000, "description": "Cap on returned text when a query is given."}
                },
                "required": ["url"]
            }
        },
        {
            "name": "sources",
            "description": "List the search sources this server can use.",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ])
}

#[derive(Deserialize)]
struct ExtractArgs {
    url: String,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default = "default_extract_chars")]
    max_chars: usize,
}

fn default_extract_chars() -> usize {
    6000
}

impl McpServer {
    pub fn new(pipeline: Arc<Pipeline>) -> Self {
        Self { pipeline }
    }

    /// Handle one JSON-RPC message, or a batch of them. Notifications produce no reply.
    pub async fn handle(&self, message: Value) -> Option<Value> {
        match message {
            Value::Array(items) if items.is_empty() => {
                Some(error_reply(Value::Null, INVALID_REQUEST, "empty batch"))
            }
            Value::Array(items) => {
                let mut replies = Vec::new();
                for item in items {
                    if let Some(r) = self.handle_one(item).await {
                        replies.push(r);
                    }
                }
                (!replies.is_empty()).then_some(Value::Array(replies))
            }
            single => self.handle_one(single).await,
        }
    }

    /// Parse a line of text, then handle it. Returns the serialized reply, if any.
    pub async fn handle_text(&self, text: &str) -> Option<String> {
        let reply = match serde_json::from_str::<Value>(text) {
            Ok(v) => self.handle(v).await,
            Err(e) => Some(error_reply(
                Value::Null,
                PARSE_ERROR,
                format!("parse error: {e}"),
            )),
        };
        reply.map(|v| v.to_string())
    }

    async fn handle_one(&self, msg: Value) -> Option<Value> {
        let Some(obj) = msg.as_object() else {
            return Some(error_reply(
                Value::Null,
                INVALID_REQUEST,
                "a message must be a JSON object",
            ));
        };
        let id = obj.get("id").cloned();
        let Some(method) = obj.get("method").and_then(Value::as_str) else {
            // A response from the client to something we never asked; nothing to do.
            return id.map(|id| error_reply(id, INVALID_REQUEST, "missing method"));
        };
        let params = obj.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = id else {
            return None; // notification (initialized, cancelled, ...)
        };

        Some(match method {
            "initialize" => {
                let asked = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let version = SUPPORTED_VERSIONS
                    .iter()
                    .find(|v| **v == asked)
                    .copied()
                    .unwrap_or(SUPPORTED_VERSIONS[0]);
                ok_reply(
                    id,
                    json!({
                        "protocolVersion": version,
                        "capabilities": {"tools": {"listChanged": false}},
                        "serverInfo": {"name": "seesea", "version": env!("CARGO_PKG_VERSION")},
                        "instructions": "Use 'search' to find and read web pages in one step, 'extract' to read a known URL, and 'sources' to see which engines are available."
                    }),
                )
            }
            "ping" => ok_reply(id, json!({})),
            "tools/list" => ok_reply(id, json!({"tools": tools()})),
            "tools/call" => self.call_tool(id, params).await,
            other => error_reply(id, METHOD_NOT_FOUND, format!("method not found: {other}")),
        })
    }

    async fn call_tool(&self, id: Value, params: Value) -> Value {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return error_reply(id, INVALID_PARAMS, "tools/call needs a tool name");
        };
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let result = match name {
            "search" => match serde_json::from_value::<SearchRequest>(args) {
                Ok(req) => match self.pipeline.search(&req).await {
                    Ok(resp) => tool_result(
                        search_markdown(&resp),
                        serde_json::to_value(&resp).unwrap_or(Value::Null),
                        false,
                    ),
                    Err(e) => tool_error(&e),
                },
                Err(e) => tool_error(&Error::InvalidRequest(e.to_string())),
            },
            "extract" => match serde_json::from_value::<ExtractArgs>(args) {
                Ok(a) => {
                    let scope = match a.scope.as_deref() {
                        None | Some("core") => Ok(Scope::Core),
                        Some("extended") => Ok(Scope::Extended),
                        Some(o) => Err(Error::InvalidRequest(format!(
                            "unknown scope '{o}', expected core or extended"
                        ))),
                    };
                    match scope {
                        Ok(scope) => match self
                            .pipeline
                            .extract(&a.url, scope, a.query.as_deref(), a.max_chars)
                            .await
                        {
                            Ok(page) => tool_result(
                                extract_markdown(&page),
                                serde_json::to_value(&page).unwrap_or(Value::Null),
                                false,
                            ),
                            Err(e) => tool_error(&e),
                        },
                        Err(e) => tool_error(&e),
                    }
                }
                Err(e) => tool_error(&Error::InvalidRequest(e.to_string())),
            },
            "sources" => {
                let names = self.pipeline.source_names();
                tool_result(names.join("\n"), json!({"sources": names}), false)
            }
            other => return error_reply(id, INVALID_PARAMS, format!("unknown tool: {other}")),
        };
        ok_reply(id, result)
    }

    /// Serve newline-delimited JSON-RPC (the MCP stdio transport) until the input ends.
    /// Requests run concurrently, so a slow search does not hold up a ping.
    pub async fn serve<R, W>(self: Arc<Self>, input: R, output: W) -> std::io::Result<()>
    where
        R: tokio::io::AsyncBufRead + Unpin,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let writer = tokio::spawn(async move {
            let mut output = output;
            while let Some(line) = rx.recv().await {
                if output.write_all(line.as_bytes()).await.is_err()
                    || output.write_all(b"\n").await.is_err()
                    || output.flush().await.is_err()
                {
                    break;
                }
            }
        });
        let mut lines = input.lines();
        let mut tasks = tokio::task::JoinSet::new();
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let (server, tx) = (Arc::clone(&self), tx.clone());
            tasks.spawn(async move {
                if let Some(reply) = server.handle_text(&line).await {
                    let _ = tx.send(reply);
                }
            });
        }
        while tasks.join_next().await.is_some() {}
        drop(tx);
        let _ = writer.await;
        Ok(())
    }
}
