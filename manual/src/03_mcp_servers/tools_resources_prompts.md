# Tools, Resources, and Prompts

A server can expose three kinds of thing, and it's worth being precise about which is which,
because newcomers reach for the wrong one constantly.

| | What it is | Who starts it | Identified by |
| --- | --- | --- | --- |
| **Tool** | A function the model can call | The model | A name |
| **Resource** | Data the client can read | The client (or the user) | A URI |
| **Prompt** | A reusable message template | The user | A name |

## Tools

We've seen tools - they're the `#[tool]` methods from the last page. They're what you reach for
when the model needs to *do* something. In practice, most servers are tools-only, and that's fine.

## Resources

Resources are for data the model should be able to *read*: a config file, a database row, a live
dashboard. They're addressed by URI, and the client lists them and reads them by name. Unlike
tools, the model doesn't call them; a harness either makes them available in context or lets the
user attach one.

On the server side you implement the trait methods and enable the capability:

```rust
impl ServerHandler for MyServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_resources()
                .build(),
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![Resource::new("file:///config.json", "config")],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        // ... return the content for request.uri ...
    }
}
```

Resource *templates* let you declare a URI pattern (`users://{user_id}/profile`) that the client
expands before calling `read_resource`. That's how you expose "many of the same kind of thing"
without listing every one.

## Prompts

Prompts are message templates a **user** invokes - think `/code-review` in a chat client. They
take typed arguments and return one or more messages. The SDK mirrors the tool macros:

```rust
#[prompt_router]
impl MyServer {
    #[prompt(name = "code_review", description = "Review code in a given language")]
    async fn code_review(
        &self,
        Parameters(args): Parameters<CodeReviewArgs>,
    ) -> Result<GetPromptResult, McpError> {
        // ... build the messages ...
    }
}

#[prompt_handler]
impl ServerHandler for MyServer { /* ... */ }
```

## Capabilities and restraint

Whatever you expose, you advertise it in `get_info()` by enabling the matching capability on
`ServerCapabilities`. A well-behaved client only calls what you've declared.

The teaching point is the same one we keep hitting: **expose the smallest surface that does the
job.** Every tool you add is a thing the model can be tricked into calling, and every description
is text the model will read and act on. A server with one well-named tool is easier to reason
about - and safer - than a server with twenty. Add resources and prompts when they earn their
keep, not because the spec has room for them.
