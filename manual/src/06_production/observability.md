# Observability

Our examples `println!` and trust you're watching. In production, nobody is watching a terminal -
and the model call is the most opaque thing in the system. Observability is how you turn that black
box back into something you can reason about after the fact.

There are three signals, and it's worth being deliberate about all three, because they answer
different questions:

* **Traces** answer *what happened, in what order, and where did the time and money go?*
* **Metrics** answer *is this normal, and should we be worried?*
* **Logs** answer *what exactly did this one request do?*

> This page is the engineering side. The compliance side - a complete, immutable record - is a
> different job with different guarantees. See [Audit](../07_governance/audit.md).

## Traces and spans: start here

If you instrument one thing, instrument spans. A **span** is a timed operation with attributes, and
spans nest into a tree: one for the request, one for each model call, one for each tool call, one
for each turn of an agent loop. That tree *is* your run. When an agent takes eleven steps and costs
fourpence, the trace tells you which step cost the money.

OpenTelemetry has **GenAI semantic conventions** (`gen_ai.*`) that standardise the names - the
model, the token counts, tool calls, and so on. Adopt them, but know they are still in *Development*
status and have been renamed more than once in a single year; pin versions and expect churn.

In Rust we already have `tracing` in the dependency tree (via `rmcp`), and it pairs with
`tracing-opentelemetry` and an OTLP exporter:

```rust
#[tracing::instrument(name = "gen_ai.chat", skip(api_key), fields(model = %model))]
async fn chat(api_key: &str, model: &str, messages: &[Message]) -> anyhow::Result<Reply> {
    // ... the request we already know how to make ...
}
```

The context propagates, so the tool call inside that function ends up as a child span without you
threading IDs by hand.

## Cost as a first-class signal

Token usage is in every response; the cost is `tokens × price`. Emit it at the call site as **both**
a span attribute and a metric, tagged by the dimensions you care about - user, tenant, feature,
session. A total on a dashboard is trivia; **attribution** is what lets you answer "who is burning
the budget?" and set a quota before somebody does. Cost and latency are the two things that will
get you paged.

## Metrics, and the alerts on top

Once you're emitting spans, the useful metrics fall out: latency (especially **time-to-first-token**,
which is what users feel), error rate, retry rate, tool-call rate, cache-hit rate, throughput. Put
thresholds on them, and have them page someone. An unread dashboard is not observability.

## Logs

Structured logs, correlated by `trace_id`/`span_id`, so you can jump from a number on a graph to the
exact request. Two rules: never log secrets, and **redact PII** - prompts and completions are
user data, and traces are not the place you want them leaking. Observability data can be *sampled*
(it costs money and has cardinality limits); audit data cannot.

## Tooling, without the wax

You can go a long way on standard pieces: `tracing` + OTLP, a collector, and a store you already
run. When you want the LLM-shaped views - prompt/response inspection, evals, cost rollups - the
open-source options are **Langfuse**, **Arize Phoenix**, and **OpenObserve**; the managed ones are
**LangSmith**, **Braintrust**, **Datadog** and friends. Many teams also put a gateway (say,
**LiteLLM**) in front so telemetry and cost are collected centrally.

The advice is the same as with governance: **start with spans, cost events, and structured logs** -
the primitives you already understand - and adopt a platform when the pain of not having one is
real.
