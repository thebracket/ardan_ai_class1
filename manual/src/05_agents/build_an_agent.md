# Let's Build An Agent

The simple agent showed us the loop in the plainest possible form: a hand-rolled `match` on the
tool name, one goal, one hard-coded corpus. Now we generalise it into something you could actually
point at a real problem.

This is the point where the pieces we've already built start to pay off. We don't need new
transport, and we don't need a new model interface - we need to wire together what's there:

* **Tools as a `ToolFactory`.** Instead of a `match` on names, we reuse the trait from `ex09`, so
  the agent can be handed any set of tools and doesn't care where they came from.
* **MCP where it helps.** Any of those tools can come from an MCP server, local or remote, exactly
  as we wired up in `ex18` and `ex19`. The agent sees one flat `tools` list; the harness routes
  each call to the right place.
* **A visible trace.** The reasoning and tool calls we've been printing all along become a real
  feature here - the transcript *is* how you debug an agent.
* **Guard rails.** A maximum number of iterations, a sensible stop condition, and errors from
  tools fed back to the model rather than crashing the run.

We'll also want to think about the goal itself. A chatbot takes a message; an agent takes a task.
Writing a good task is its own small craft: say what "done" looks like, tell it what it must not
do, and don't over-specify the steps - that's what you're paying the model to work out.

The earlier warnings still apply, and they get sharper as the agent gets more capable. Keep the
deterministic work in code. Give it the smallest toolset that does the job. And remember that
every tool you hand an agent is also a thing a prompt injection can try to reach - which is
exactly why we covered provenance before we got here.
