# Your Results Will Differ

Here's something to internalise before we call our first model: it isn't a function. Ask the
same question twice and you may get two different answers. Ask again tomorrow and the answer
may differ again, because the thing behind the model's name can change without the name
changing.

This page exists so that when that happens - and it will - you don't spend the afternoon
debugging perfectly good code.

## Why It Happens

Language models generate text one token at a time, by sampling from a probability
distribution. That's the whole explanation; everything else is bookkeeping:

* **Sampling.** Most providers default to a non-zero temperature, which means "pick from the
  plausible options, weighted". We don't set a temperature in our requests, so we get the
  provider's default - and it's a roll of the dice.
* **Reasoning.** Our requests turn reasoning on, so the model thinks before it speaks. The
  reasoning trace varies run to run, and a different train of thought can lead to a different
  final answer.
* **The model may change.** `deepseek/deepseek-v4.1-flash` is a name, not a frozen artifact.
  Providers update models, and a service like OpenRouter can route the same name to different
  hosting backends. The same request can reach slightly different weights a month from now.
* **A seed is a nudge, not a promise.** Some APIs accept a seed to make outputs more
  repeatable. It reduces variation on the same backend; it doesn't freeze the model version,
  and it doesn't guarantee the same result twice.

## What It Means in Practice

The transcripts in this book are real runs from one machine, on one day. They are "what it
looked like", not "what it must look like". Yours will differ, and when I teach this material
live, the demo will differ from the book too. That's the medium, not a mistake.

The places it bites in this book:

* **Tool selection.** The model might call a different tool than you expected, pass different
  arguments, or decide no tool is needed at all.
* **Prose.** Wording, length, formatting, emoji - none of it is stable enough to compare.
* **Multi-turn conversations.** Small differences early compound; by turn five the
  conversation can be somewhere else entirely.
* **Streaming.** Chunk boundaries vary, so don't diff raw bytes - you'll see differences that
  mean nothing.
* **Sensitive demos.** Some later examples deliberately switch models, because a demo that
  works on one model may not land on another. Prompt injection is the obvious one.

## What Stays Stable

It helps to know which half of the system is which. The model is non-deterministic; our side
of the wire is not:

* The JSON we send - messages, tool definitions, ids - is the same for the same input.
* The protocol is deterministic: request ids match responses, tool call ids identify results,
  and message order is whatever our code says it is.
* Your tool code runs the same way every time. `roll_dice` rolls a different number, but it
  rolls it the same way.

So when something looks wrong, look at the JSON first. If the wire is right and the model did
something odd, that's non-determinism. If the wire is wrong, that's a bug - and it's yours.

## What To Do About It

* **Read structure, not prose.** "Did a tool call happen? Which tool? What were the
  arguments?" are answerable questions. "Did it say 'certainly'?" is not.
* **Steer the demo.** `Use the get-sum tool.` is a perfectly honest prompt. If you're
  demonstrating plumbing, you're allowed to remove the model's choices from the equation.
* **Retry before you debug.** If a prompt works most of the time, that's a prompt problem, not
  a code problem. If it fails consistently, the JSON will tell you why.
* **Don't test the model; test your code.** Assert on parsing, routing and the shapes you
  control. Leave exact wording out of the test suite - otherwise you're writing flaky tests
  for a component you don't own.
* **Keep decisions in code where they matter.** The `ex16_structured` lesson, again: the model
  describes and converses; thresholds, arithmetic and policy belong in Rust, where you can
  test them.

None of this makes the model useless - it makes it a colleague rather than a calculator. Plan
for variation and it stops being a surprise.
