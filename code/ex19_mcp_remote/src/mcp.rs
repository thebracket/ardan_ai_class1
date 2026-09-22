//! A small MCP client for the Streamable HTTP transport.
//!
//! It's the same protocol as `ex17_mcp_client`, with different plumbing: one
//! endpoint, one POST per message, and the answer arrives as either a JSON
//! body or a short request-scoped SSE stream. Servers from before 2026-07-28
//! may also hand out a session id, which every later request has to carry.

use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde_json::{Value, json};

/// The revision this client prefers. It's the newest one we know, and the
/// first with no sessions at all.
const PREFERRED_VERSION: &str = "2026-07-28";

/// The last revision before the protocol went stateless.
const LEGACY_VERSION: &str = "2025-11-25";

/// The first Streamable HTTP revision. It predates the
/// `MCP-Protocol-Version` header, so we never send that to it.
const FIRST_HTTP_VERSION: &str = "2025-03-26";

/// What a server told us about itself during discovery.
pub struct ServerDescription {
    pub protocol_version: String,
    pub name: String,
    pub version: String,
    pub capabilities: Value,
}

pub struct McpClient {
    http: reqwest::Client,
    url: String,
    next_id: u64,
    /// The revision we agreed on with the server.
    protocol_version: String,
    /// 2026-07-28 servers carry version and identity in `_meta` on every
    /// request; legacy servers negotiated once, at connect time.
    modern: bool,
    /// Set once a legacy handshake has finished, so we know when it's legal
    /// to start sending the version header.
    handshake_complete: bool,
    /// Legacy servers may hand out a session on the initialize response;
    /// every later request has to carry it back.
    session_id: Option<String>,
}

impl McpClient {
    pub async fn connect(url: &str) -> Result<Self> {
        println!("Connecting to {url}");

        Ok(McpClient {
            http: reqwest::Client::builder().build()?,
            url: url.to_string(),
            next_id: 1,
            protocol_version: PREFERRED_VERSION.to_string(),
            modern: true,
            handshake_complete: false,
            session_id: None,
        })
    }

    /// Find out who the server is and which revision it speaks, falling back
    /// to the legacy handshake if it has never heard of `server/discover`.
    pub async fn discover(&mut self) -> Result<ServerDescription> {
        println!("Trying `server/discover` ({PREFERRED_VERSION})...");
        let probe = self.request("server/discover", json!({})).await;

        match probe {
            Ok(response) if response.get("error").is_none() => {
                // A modern server answered, which means it accepted our version.
                println!("  Modern server: no handshake, no session.");
                let result = &response["result"];
                let info = &result["_meta"]["io.modelcontextprotocol/serverInfo"];
                Ok(ServerDescription {
                    protocol_version: self.protocol_version.clone(),
                    name: info["name"].as_str().unwrap_or("unknown").to_string(),
                    version: info["version"].as_str().unwrap_or("unknown").to_string(),
                    capabilities: result["capabilities"].clone(),
                })
            }
            Ok(response) => {
                let error = &response["error"];
                let code = error["code"].as_i64().unwrap_or(0);
                match code {
                    // -32022: a modern server that can't speak our revision.
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
                    // The other reserved modern errors mean the server
                    // understood us but didn't like what we sent.
                    -32020 | -32021 => anyhow::bail!("server rejected discovery: {error}"),
                    // Anything else is a legacy server answering a request it
                    // doesn't understand `-32600`, `-32602`, `-32601`, ...
                    _ => {
                        println!("  Not supported ({code}); falling back to the old handshake.");
                        self.initialize_legacy().await
                    }
                }
            }
            Err(error) => {
                // Over HTTP a legacy server may answer with a 400/404/405 and
                // no JSON-RPC body at all. The spec says an empty or
                // unrecognised body is the cue to fall back; a genuinely
                // unreachable server will fail again, loudly, below.
                println!("  Modern probe refused ({error}); falling back.");
                self.initialize_legacy().await
            }
        }
    }

    /// The pre-2026 handshake: version and capabilities are negotiated once,
    /// and legacy servers often tie a session to the result.
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

        // The server picks the exact revision; it may be older than the one
        // we asked for.
        self.protocol_version = result["protocolVersion"]
            .as_str()
            .unwrap_or(LEGACY_VERSION)
            .to_string();
        self.handshake_complete = true;

        // The connection is negotiated; this tells the server we're ready
        // for normal traffic.
        self.notify("notifications/initialized", json!({})).await?;

        let info = &result["serverInfo"];
        Ok(ServerDescription {
            protocol_version: self.protocol_version.clone(),
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

            match result["nextCursor"].as_str() {
                Some(next) => cursor = Some(next.to_string()),
                None => break,
            }
        }

        Ok(tools)
    }

    /// Ask the server to run one of its tools.
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
        Ok(result_of(&response, "tools/call")?.clone())
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

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let (status, messages) = self.post(&message).await?;

        let mut error_envelope = None;
        for message in messages {
            if message["id"] == json!(id) {
                return Ok(message);
            }
            if message.get("error").is_some() {
                // Some servers answer a rejection with a string id, or none
                // at all. Keep it: it's the only explanation we're getting.
                error_envelope = Some(message);
                continue;
            }
            if message.get("id").is_none() {
                println!("[server notification] {}", serde_json::to_string(&message)?);
                continue;
            }
            println!("[stray response] {}", serde_json::to_string(&message)?);
        }

        if let Some(error) = error_envelope {
            return Ok(error);
        }
        anyhow::bail!("no response to `{method}` (HTTP {status})")
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.post(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
        .await?;
        Ok(())
    }

    /// One POST, zero or more messages back.
    async fn post(&mut self, message: &Value) -> Result<(StatusCode, Vec<Value>)> {
        let method = message["method"].as_str().unwrap_or_default();

        let mut request = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json")
            // We support both shapes of answer, and modern servers may pick
            // either one per request.
            .header("Accept", "application/json, text/event-stream")
            .json(message);

        if self.modern {
            // Modern servers mirror the interesting body fields into headers,
            // so gateways can route without parsing the body.
            request = request
                .header("MCP-Protocol-Version", self.protocol_version.as_str())
                .header("Mcp-Method", method);
            if let Some(name) = message["params"]["name"].as_str() {
                request = request.header("Mcp-Name", name);
            }
        } else if self.handshake_complete && self.protocol_version != FIRST_HTTP_VERSION {
            // 2025-06-18 and later ask for the negotiated version on every
            // request after the handshake. The initialize POST itself is
            // exempt, which is why this waits for `handshake_complete`.
            request = request.header("MCP-Protocol-Version", self.protocol_version.as_str());
        }

        if let Some(session) = &self.session_id {
            request = request.header("Mcp-Session-Id", session);
        }

        let response = request.send().await.context("HTTP request failed")?;
        let status = response.status();

        // A legacy server can hand us a session at any point. Keep it: every
        // later request has to carry it back.
        if let Some(session) = response.headers().get("Mcp-Session-Id") {
            if let Ok(session) = session.to_str() {
                self.session_id = Some(session.to_string());
            }
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();

        if status == StatusCode::OK && content_type.starts_with("text/event-stream") {
            // A request-scoped SSE stream: any notifications first, then the
            // response, then the stream closes.
            let body = response.text().await?;
            Ok((status, parse_sse(&body)?))
        } else {
            // A single JSON object - or, for a notification, a 202 with no
            // body at all.
            let body = response.text().await?;
            if body.trim().is_empty() {
                return Ok((status, Vec::new()));
            }
            let message = serde_json::from_str::<Value>(&body)
                .with_context(|| format!("HTTP {status}, expected JSON, got: {body}"))?;
            Ok((status, vec![message]))
        }
    }
}

/// Pull the JSON-RPC messages out of an SSE body.
///
/// One message per event, with the JSON sitting on one or more `data:` lines.
/// We do this by hand for a reason: SSE allows either LF or CRLF line endings,
/// and not every helper library copes with both. This is the same `data:`
/// parsing we did back in `ex04_streaming`.
fn parse_sse(body: &str) -> Result<Vec<Value>> {
    let mut messages = Vec::new();
    let mut data = String::new();

    // `lines()` treats LF and CRLF the same, which is the point.
    for line in body.lines() {
        if line.is_empty() {
            // A blank line dispatches the event we've been accumulating.
            if !data.is_empty() {
                messages.push(serde_json::from_str(&data)?);
                data.clear();
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        }
        // `event:`, `id:`, `retry:` and `:` comments don't matter to us.
    }

    // A final event with no trailing blank line is still an event.
    if !data.is_empty() {
        messages.push(serde_json::from_str(&data)?);
    }

    Ok(messages)
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
