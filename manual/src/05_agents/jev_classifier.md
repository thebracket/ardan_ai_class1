# A Jev Classifier in Rust

[Last page](./classify_dont_generate.md) argued that most "structured output" is really a
decision, and that decisions deserve a classifier. This page is the worked example.

> This is in the repo as `code/ex21_jev`.

**Jev** is TypeSafe AI's System One decision model. You send it a `state` (any messy text) and a
set of **typed questions**, and it returns typed answers with calibrated probabilities. It never
generates prose, so there is nothing to parse: the answer is already the shape your code wants.

It's a hosted model, so there's no local, keyless version - but it's served on **OpenRouter** as
`typesafe/jev-1.13`, which is convenient, because we already have an `OPENROUTER_KEY` from every
other example. No separate signup, waitlist, or TypeSafe account. There's no built-in Jev in
OpenCode, but the `jev-mcp` / `jev-code` MCP servers and skills can wire it in, and
`jev-gateway` routes your whole agent through it, if you want the model available to your coding
agent as a tool.

## The API, in one breath

```http
POST https://openrouter.ai/api/v1/systemone
Authorization: Bearer $OPENROUTER_KEY
Content-Type: application/json
```

The body has three fields: `model` (`"typesafe/jev-1.13"`), `state` (the text to judge), and
`questions` (a map from an id *you* choose to a question object). There are three question types:

| Type | Asks | Returns |
| --- | --- | --- |
| `choice` | Pick one of up to 255 labelled options | `choice`, `probabilities`, `confidence` |
| `score` | Place the input on an ordered scale | `score`, `probabilities`, `confidence` |
| `noul` | Is this true? | `noul` (a probability from 0 to 1) |

Mix as many as you like in one call; they're evaluated in parallel, so adding questions barely
changes the latency. The endpoint returns something like this - note that it is *already* typed:

```json
{
  "model": "typesafe/jev-1.13-20260917",
  "provider": "TypeSafe",
  "answers": {
    "department": {
      "type": "choice",
      "choice": "billing",
      "confidence": 0.61,
      "probabilities": { "returns": 0.24, "shipping": 0.15, "billing": 0.61 }
    },
    "wants_human": { "type": "noul", "noul": 0.99 }
  },
  "usage": { "input_tokens": 328, "output_tokens": 34, "cost": 0.00003 }
}
```

> OpenRouter doesn't list Jev in `GET /api/v1/models`, because that list is for text models; you
> reach Jev through the System One / decisions endpoint instead of `/chat/completions`. If you'd
> rather use TypeSafe directly, the same request goes to `https://api.typesafe.ai/v1/systemone`
> with a TypeSafe key.

## The rules for writing good questions

This is the bit people get wrong, and it's very different from prompting an LLM. Jev wants *small,
precise* questions, and the structure you give them matters more than flowery wording:

* **One judgement per question.** If you want to know both urgency and sentiment, ask two
  questions and combine them in code. A question that weighs several factors at once is a question
  whose answer means nothing.
* **A Choice needs a description for every option**, and the descriptions should separate the
  options from each other. Always include an `other` / `none of the above` so the model is never
  forced to pick a label that doesn't fit.
* **Phrase a Noul so that a high value means yes.** "Is the customer asking for a human?" is
  right; "Is the customer *not* asking for a human?" is a bug waiting to happen. Add `criteria`
  with `true` and `false` descriptions when the boundary is fuzzy.
* **Batch your questions.** One request with five questions is about as fast as one request with
  one, and much cheaper than five calls.
* **Keep state focused.** Send what the decision needs, trimmed of anything else. Jev is good at
  ignoring noise, but you still pay for the tokens.
* **The ids are yours.** `department` and `wants_human` are just keys; the model never sees them.

## What the example does

`ex21_jev` triages a support ticket. It asks a `choice` (which team) and a `noul` (is the customer
asking for a human), then routes with an ordinary `match`. The ticket comes from the command line
(with a clear-cut default), so you can try as many as you like:

```rust
let request = json!({
    "model": "typesafe/jev-1.13",
    "state": ticket,
    "questions": {
        "department": {
            "type": "choice",
            "instructions": "Which team should handle this?",
            "criteria": {
                "returns": "Exchanges, wrong or damaged items",
                "shipping": "Delivery status, delays, lost packages",
                "billing": "Charges, invoices, payment problems",
                "other": "Fits none of the above"
            }
        },
        "wants_human": {
            "type": "noul",
            "instructions": "Is the customer asking to speak to a human?",
            "criteria": {
                "true": "Asks for a person, an agent, or someone real",
                "false": "No request to talk to a person"
            }
        }
    }
});
```

Because the answer is typed, it deserializes straight into our structs, and the decision is code:

```rust
if department.confidence < 0.5 {
    // Not sure enough to act. A person decides.
    println!("-> confidence too low; sending to manual triage");
    return Ok(());
}

match Team::from_label(&department.choice) {
    Team::Returns => println!("-> route to the returns team"),
    Team::Billing => println!("-> route to the billing team"),
    // ...
}
```

Compare that with the previous page. There is no schema to validate, no retry-on-parse-failure
loop, and no way for the model to return `"billing_something"` and quietly break a `match`. The
answer is constrained by the request, so the failure mode simply doesn't exist.

> The probability isn't a guarantee of *correctness* - it's how concentrated the distribution is.
> A low `confidence` means "the model sees several plausible answers", which is exactly when you
> want a human. That threshold is yours, and it's code.

## Running it

It uses the shared `OPENROUTER_KEY`, so from the crate directory it just runs. The ticket is a
command-line argument, so you can try different ones:

```bash
cd code/ex21_jev
cargo run
```

That default is a clear-cut ticket, and shows the whole thing working:

```text
ticket: I've been charged twice for my subscription this month, and I have asked three times now - can I please just talk to a real person?
department: billing (confidence 1.00)
wants_human: 0.99

-> route to the billing team
-> customer asked for a human; escalate to an agent
```

Pass a murkier ticket - one that's plausibly two teams at once - and the confidence gate fires
instead of guessing:

```bash
cargo run -- "My running shoes arrived in the wrong size, and I can see two charges on my card. I have asked three times now - can I please just talk to a real person?"
```

```text
department: returns (confidence 0.38)
  returns: 0.54
  billing: 0.37
  other: 0.09
  shipping: 0.00

wants_human: 0.99

-> confidence too low; sending to manual triage
```

Jev is fast (roughly 70-500ms) and cheap per decision - each call reports its own `usage.cost`,
and the example prints it (both runs above came in under $0.00002). That low cost is the whole
reason to reach for it instead of a frontier model when all you needed was a `match`.
