//! Translating MCP tools into the shape our chat loop already understands.
//!
//! `McpClient` speaks the protocol; `McpTools` is the adapter. It owns the
//! client (and therefore the server process), holds the tool list translated
//! into `ToolDefinition`s, and routes a model's tool call back out to the
//! server that knows how to run it.

use std::collections::HashMap;

use anyhow::Result;
use serde_json::{Value, json};

use crate::chatbot::ToolDefinition;
use crate::mcp::{McpClient, ServerDescription};

/// One MCP server, wearing the same clothes as our hand-written tools.
pub struct McpTools {
    client: McpClient,
    server: ServerDescription,
    tools: Vec<ToolDefinition>,
    /// The name we showed the model -> the name the server knows.
    server_names: HashMap<String, String>,
}

impl McpTools {
    /// Spawn a server, find out what it speaks, and translate its tools.
    pub async fn connect(namespace: &str, command: &str, args: &[&str]) -> Result<Self> {
        let mut client = McpClient::spawn(command, args).await?;
        let server = client.discover().await?;

        let mut tools = Vec::new();
        let mut server_names = HashMap::new();
        for raw in client.list_tools().await? {
            let definition = translate(namespace, &raw);
            server_names.insert(
                definition.name.clone(),
                raw["name"].as_str().unwrap_or_default().to_string(),
            );
            tools.push(definition);
        }

        Ok(Self {
            client,
            server,
            tools,
            server_names,
        })
    }

    pub fn server(&self) -> &ServerDescription {
        &self.server
    }

    /// The translated tools, ready to join the request we send the model.
    pub fn tools(&self) -> Vec<ToolDefinition> {
        self.tools.clone()
    }

    /// Run one MCP tool and turn its result into the `(key, value)` pair our
    /// session wraps into a tool message.
    pub async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String) {
        let Some(server_name) = self.server_names.get(tool_name).cloned() else {
            return ("error".to_string(), format!("Unknown tool: {tool_name}"));
        };

        // The model hands us arguments as a JSON *string*; MCP wants a JSON
        // *object*. If the model sent nonsense, send an empty object and let
        // the server explain what's missing.
        let arguments = serde_json::from_str::<Value>(arguments).unwrap_or_else(|_| json!({}));

        match self.client.call_tool(&server_name, arguments).await {
            Ok(result) => render_result(&result),
            // A protocol-level failure (the server died, the request was
            // malformed) is data too: hand it to the model rather than
            // panicking the chat loop.
            Err(error) => ("error".to_string(), format!("MCP call failed: {error}")),
        }
    }
}

/// MCP sends JSON Schema; our `ToolDefinition` can only say `(name, type)` plus
/// `required`. Take what fits and leave the rest - that's the trade-off we
/// talked about before writing this chapter.
fn translate(namespace: &str, raw: &Value) -> ToolDefinition {
    let schema = &raw["inputSchema"];

    let required = schema["required"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let parameters = schema["properties"]
        .as_object()
        .map(|properties| {
            properties
                .iter()
                .map(|(name, property)| {
                    // A nested schema becomes a bare `"object"` or `"array"`;
                    // the server still validates the real thing on the way in.
                    let kind = property["type"].as_str().unwrap_or("string");
                    (name.clone(), kind.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    ToolDefinition {
        name: format!("{namespace}__{}", raw["name"].as_str().unwrap_or("unnamed")),
        description: raw["description"].as_str().unwrap_or("").to_string(),
        parameters,
        required,
    }
}

/// Turn an MCP result into the `(key, value)` pair our session wraps into a
/// tool message.
fn render_result(result: &Value) -> (String, String) {
    // A tool that ran and failed is still a result. The model reads the
    // message and gets a chance to fix its arguments.
    if result["isError"].as_bool() == Some(true) {
        return ("error".to_string(), content_text(result));
    }

    // Structured content is the good stuff: a real object the model can read
    // field by field, which our session will parse back out of the string.
    if let Some(structured) = result.get("structuredContent") {
        return ("result".to_string(), structured.to_string());
    }

    ("result".to_string(), content_text(result))
}

/// Flatten an MCP result's content blocks into text. Anything we don't know how
/// to render gets described instead, so the model at least knows it existed.
fn content_text(result: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(items) = result["content"].as_array() {
        for item in items {
            match item["type"].as_str() {
                Some("text") => parts.push(item["text"].as_str().unwrap_or("").to_string()),
                Some(other) => parts.push(format!(
                    "[the tool also returned {other} content, which this client ignores for now]"
                )),
                None => {}
            }
        }
    }

    if parts.is_empty() {
        parts.push("(the tool returned no content)".to_string());
    }
    parts.join("\n")
}
