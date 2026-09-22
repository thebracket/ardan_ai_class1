# A Server to Play With

> **Draft — planning notes.**

In chapter three we borrowed the official "everything" server to practise discovery on. It is
the best *first* server in the ecosystem, and a terrible *only* server: it exists to exercise
the protocol, not to do anything useful. So this chapter is a short tour of where servers
actually come from - and of the tool we use to poke at them without writing a line of code.

## The Everything Server, Again

We've already met it, but here's the command we'll keep using, because it's the one that always
works for a quick experiment:

```bash
npx -y @modelcontextprotocol/server-everything
```

It's a reference implementation from the MCP steering group: tools, resources, prompts,
logging, tasks, and a deliberate attempt to hit every corner of the spec. The project is blunt
that these servers are for *learning*: "not production-ready solutions". Treat it as a
laboratory rat, not a pet.

## The Reference Servers

The steering group maintains a small set of official reference servers - everything, fetch,
filesystem, git, memory, sequential thinking, and time - at
<https://github.com/modelcontextprotocol/servers>. The TypeScript ones run through `npx`; the
Python ones (like git) run through `uvx` or `pip`.

Two of them are worth five minutes each, because they show opposite ends of the design space.

**Memory** needs no configuration at all:

```bash
npx -y @modelcontextprotocol/server-memory
```

It's a knowledge graph: the model can create entities, relate them, and search them later. Its
tools are a nice cross-section of a "real" server:

```text
create_entities     create_relations    add_observations
delete_entities     delete_observations delete_relations
read_graph          search_nodes        open_nodes
```

**Filesystem** takes a directory and promises to stay inside it:

```bash
npx -y @modelcontextprotocol/server-filesystem /path/to/a/directory
```

```text
read_file           read_text_file      read_media_file
read_multiple_files write_file          edit_file
create_directory    list_directory      directory_tree
move_file           search_files        get_file_info
list_allowed_directories
```

That argument *is* the security model - the server is allowed to touch one subtree, and the
tool list says `list_allowed_directories` so the model can see the fence it's standing in.
It's a good design to study before we reach **MCP Security and Provenance**.

Beyond the official set, the ecosystem lives at the [MCP Registry](https://registry.modelcontextprotocol.io/).
The same rules apply as to any dependency you didn't write: it's somebody else's code, it gets
to describe its own tools, and it will be talking to a model. Pin versions, read the tool list,
and start small.

## Rust-Native Servers

If you'd rather not depend on Node at all, the [Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
ships server examples, including two that are ideal for testing a client: a counter and a
calculator. Clone the SDK and run one:

```bash
git clone https://github.com/modelcontextprotocol/rust-sdk
cd rust-sdk
cargo run -p mcp-server-examples --example servers_counter_stdio
```

The example names carry a `servers_` prefix (`servers_counter_stdio`,
`servers_calculator_stdio`, `servers_memory_stdio`, and friends), and the SDK's examples
directory has more: stdio, streamable HTTP, prompts, elicitation, tasks. This is also where we
start when we write our own server in **Writing MCP Servers**.

## The Inspector

Poking servers with raw JSON gets old (we'll do it for one chapter anyway, because it's
instructive). The official [MCP Inspector](https://github.com/modelcontextprotocol/inspector)
is the GUI version:

```bash
npx @modelcontextprotocol/inspector
```

It opens a web UI on <http://localhost:6274>. You give it a server: a command for stdio, or a
URL for Streamable HTTP. From there you can browse capabilities, list and call tools with
arguments, read resources, and watch the message log. It is the fastest way to answer "what
does this server actually expose?", and it's what I reach for before wiring a new server into
anything.

The Inspector is on its second major version and wants Node `>= 22.19.0`; on older Node, npm
will happily install it and then fail in confusing ways, so check `node --version` first if
things look strange. (If `npx` hands you a cached v1, pin it: `npx @modelcontextprotocol/inspector@latest`.)
There are three ways in:

* `npx @modelcontextprotocol/inspector` - the web UI (the default).
* `npx @modelcontextprotocol/inspector --tui` - an interactive terminal UI, for when you're
  already living in a terminal.
* `npx @modelcontextprotocol/inspector --cli` - scriptable, one operation per run.

The CLI is worth knowing about because it's how you check a server in CI, or in a smoke test
before class. Everything before a `--` is the server command (flags and all); everything after
it is for the Inspector:

```bash
npx @modelcontextprotocol/inspector --cli npx -y @modelcontextprotocol/server-everything \
  -- --method tools/list
```

...which spits out the full `tools/list` result as JSON. Calling a tool is one more flag:

```bash
npx @modelcontextprotocol/inspector --cli npx -y @modelcontextprotocol/server-everything \
  -- --method tools/call --tool-name get-sum --tool-arg a=2 --tool-arg b=3
```

```json
{
  "content": [
    {
      "type": "text",
      "text": "The sum of 2 and 3 is 5."
    }
  ]
}
```

That's the whole point of this page, really: a server is a thing you can interrogate. Before
you connect it to a model, connect it to the Inspector.

## The Ecosystem Moves

Two warnings from someone who has re-recorded this material more than once:

* **Check your tools before class.** Servers get archived, renamed, retransported, and
  re-specced. Everything in this chapter worked when this page was written; the ecosystem
  treats changelogs as a suggestion.
* **Don't trust a server because it's popular.** The registry is a list, not a vetting
  process. The tool descriptions you're about to feed to a model are written by whoever
  published the server, and we'll see exactly why that matters later.

## References

* Reference servers: <https://github.com/modelcontextprotocol/servers>
* MCP Registry: <https://registry.modelcontextprotocol.io/>
* Rust SDK: <https://github.com/modelcontextprotocol/rust-sdk>
* MCP Inspector: <https://github.com/modelcontextprotocol/inspector>
