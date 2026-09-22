//! A very small MCP client.
//!
//! It does three things: launches a stdio server, works out which revision
//! of the protocol the server speaks, and lists its tools. Everything else
//! we need later - calling tools, feeding results to the model - builds on
//! this. Deliberately primitive: one request in flight, no timeouts, no
//! reconnection. We'll add the grown-up parts as we need them.

use std::process::Stdio;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// The protocol revision this client prefers to speak. It's the newest one
/// we know about, and the one that turned MCP stateless.
const PREFERRED_VERSION: &str = "2026-07-28";

/// The last revision before MCP went stateless. Servers on this revision
/// (or older) still expect the old `initialize` handshake.
const LEGACY_VERSION: &str = "2025-11-25";

/// What a server told us about itself during discovery.
pub struct ServerDescription {
    pub protocol_version: String,
    pub name: String,
    pub version: String,
    pub capabilities: Value,
}

pub struct McpClient {
    /// Held only so the server dies when we do (`kill_on_drop`).
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    /// The revision we agreed on with the server.
    protocol_version: String,
    /// 2026-07-28 servers expect our version and identity on every request,
    /// in `_meta`; legacy servers negotiated that once, at connect time.
    modern: bool,
}

impl McpClient {
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        println!("Launching {} {}", command, args.join(" "));

        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // The protocol owns stdout. Server logs go to stderr, which we
            // inherit so they show up on our terminal instead of corrupting
            // the conversation.
            .stderr(Stdio::inherit())
            // The server is our child: when the client goes, it goes.
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("failed to run `{command}`"))?;

        let stdin = child.stdin.take().context("server has no stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("server has no stdout")?);

        Ok(McpClient {
            _child: child,
            stdin,
            stdout,
            next_id: 1,
            protocol_version: PREFERRED_VERSION.to_string(),
            modern: true,
        })
    }

    /// Find out who the server is and which revision it speaks, falling back
    /// to the legacy handshake if it has never heard of `server/discover`.
    pub async fn discover(&mut self) -> Result<ServerDescription> {
        println!("Trying `server/discover` ({PREFERRED_VERSION})...");
        let response = self.request("server/discover", json!({})).await?;

        if response.get("error").is_none() {
            // A modern server answered, which means it accepts our version.
            let result = &response["result"];
            let info = &result["_meta"]["io.modelcontextprotocol/serverInfo"];
            return Ok(ServerDescription {
                protocol_version: self.protocol_version.clone(),
                name: info["name"].as_str().unwrap_or("unknown").to_string(),
                version: info["version"].as_str().unwrap_or("unknown").to_string(),
                capabilities: result["capabilities"].clone(),
            });
        }

        let error = &response["error"];
        let code = error["code"].as_i64().unwrap_or(0);
        match code {
            // -32022: a modern server that can't speak our revision, telling
            // us which ones it can. The only other revision we know is the
            // legacy handshake, so take that if it's on offer.
            -32022 => {
                let supported = supported_versions(error);
                println!("  Version rejected; server speaks {supported:?}");
                if supported.contains(&LEGACY_VERSION) {
                    return self.initialize_legacy().await;
                }
                anyhow::bail!(
                    "server speaks {supported:?}, but this client only speaks \
                     {PREFERRED_VERSION} and {LEGACY_VERSION}"
                );
            }
            // The other reserved modern errors mean the server understood us
            // but didn't like what we sent; falling back won't help.
            -32020 | -32021 => anyhow::bail!("server rejected discovery: {error}"),
            // Anything else - in practice -32601, "Method not found" - is a
            // legacy server that has never heard of `server/discover`.
            _ => {
                println!("  Not supported ({code}); falling back to the old handshake.");
                self.initialize_legacy().await
            }
        }
    }

    /// The pre-2026 handshake: version and capabilities are negotiated once,
    /// and the connection remembers them from then on.
    async fn initialize_legacy(&mut self) -> Result<ServerDescription> {
        self.modern = false;
        self.protocol_version = LEGACY_VERSION.to_string();
        println!("Initializing the 2025 way ({LEGACY_VERSION})...");

        let response = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": LEGACY_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "workshop-mcp-client", "version": "1.0.0" }
                }),
            )
            .await?;

        let result = result_of(&response, "initialize")?;
        let info = &result["serverInfo"];

        // The connection is negotiated; this tells the server we're ready
        // for normal traffic.
        self.notify("notifications/initialized", json!({})).await?;

        Ok(ServerDescription {
            protocol_version: result["protocolVersion"]
                .as_str()
                .unwrap_or(LEGACY_VERSION)
                .to_string(),
            name: info["name"].as_str().unwrap_or("unknown").to_string(),
            version: info["version"].as_str().unwrap_or("unknown").to_string(),
            capabilities: result["capabilities"].clone(),
        })
    }

    /// Ask for every tool the server has, following `nextCursor` pages.
    pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let params = match &cursor {
                Some(cursor) => json!({ "cursor": cursor }),
                None => json!({}),
            };
            let response = self.request("tools/list", params).await?;
            let result = result_of(&response, "tools/list")?;

            if let Some(page) = result["tools"].as_array() {
                tools.extend(page.iter().cloned());
            }

            // Long lists arrive in pages; keep asking until the server stops
            // handing us cursors.
            match result["nextCursor"].as_str() {
                Some(next) => cursor = Some(next.to_string()),
                None => break,
            }
        }

        Ok(tools)
    }

    /// Ask the server to run one of its tools. Returns the raw MCP result
    /// object: `content`, maybe `structuredContent`, and `isError`.
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let response = self
            .request(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": arguments,
                }),
            )
            .await?;

        // A JSON-RPC error is the protocol refusing the request, not the tool
        // running and failing. That distinction matters to what we show the
        // model, and `mcp_tools.rs` handles both.
        if let Some(error) = response.get("error") {
            anyhow::bail!("{error}");
        }
        Ok(response["result"].clone())
    }

    /// Send a request and wait for the response that carries its id. Returns
    /// the whole JSON-RPC envelope, errors included: a server saying "no" is
    /// still the server answering us.
    async fn request(&mut self, method: &str, mut params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        if self.modern {
            // Modern requests stand alone: the server may know nothing about
            // any previous request, so we repeat ourselves every time.
            params["_meta"] = json!({
                "io.modelcontextprotocol/protocolVersion": self.protocol_version,
                "io.modelcontextprotocol/clientInfo": {
                    "name": "workshop-mcp-client",
                    "version": "1.0.0"
                },
                "io.modelcontextprotocol/clientCapabilities": {}
            });
        }

        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))
        .await?;

        loop {
            let message = self.read_message().await?;

            // Notifications have no `id`, and can arrive at any time. They
            // aren't the answer we're waiting for, but it's worth seeing them.
            if message.get("id").is_none() {
                println!("[server notification] {}", serde_json::to_string(&message)?);
                continue;
            }

            if message["id"] == json!(id) {
                return Ok(message);
            }

            // A response to something else. We only keep one request in
            // flight, so this shouldn't happen - but don't lose it silently.
            println!("[stray response] {}", serde_json::to_string(&message)?);
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
        .await
    }

    async fn send(&mut self, message: Value) -> Result<()> {
        // stdio framing: one JSON object per line, and no newlines inside it.
        let mut line = serde_json::to_string(&message)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Value> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line).await?;
        if bytes == 0 {
            anyhow::bail!("the server closed its stdout");
        }
        serde_json::from_str(&line)
            .with_context(|| format!("server sent a non-JSON line: {line:?}"))
    }
}

/// Pull the `result` out of a response envelope, or explain what the server
/// said instead.
fn result_of<'a>(response: &'a Value, method: &str) -> Result<&'a Value> {
    match response.get("result") {
        Some(result) => Ok(result),
        None => anyhow::bail!("{method} failed: {}", response["error"]),
    }
}

fn supported_versions(error: &Value) -> Vec<&str> {
    error["data"]["supported"]
        .as_array()
        .map(|versions| versions.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}
