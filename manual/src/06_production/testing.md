# Testing

Every test we've written so far assumed a comfortable lie: one input, one output. Hand the code
`2 + 2` and you get `4`, every time. An LLM breaks that assumption. The same prompt can come back
three different ways, and all three can be correct - or the third can be nonsense.

So testing a model-driven system is two jobs, not one:

1. **Tame the non-determinism** so most of your suite is ordinary, fast, and repeatable.
2. **Assert on the rest statistically**, because you can't assert on a single run.

## Keep the model out of as much as possible

The most valuable habit is structural: keep the actual model call in a thin, isolated adapter, and
keep the decisions in ordinary code. Then almost everything is testable the way you already test
things - no model involved.

That also gives you somewhere to put a **deterministic seam**. Inject the client, so tests can swap
in a fake:

* run at `temperature 0`, and a fixed seed where the provider supports one;
* **pin the model version**, and record it, because "the model changed" is a real source of
  regressions;
* **record and replay** real interactions (VCR-style) so CI never talks to a provider.

With those in place, the vast majority of your tests are dull, deterministic unit tests - which is
exactly what you want.

## Assert statistically when you can't help it

Where the model *is* the thing under test, stop reaching for equality and start measuring:

```rust
// Run the same case many times and assert on the *shape* of the results.
let outcomes: Vec<bool> = (0..20)
    .map(|_| run_once().await.unwrap())
    .collect();
let pass_rate = outcomes.iter().filter(|ok| **ok).count() as f64 / 20.0;
assert!(pass_rate >= 0.9, "pass rate {pass_rate:.2} fell below 0.9");
```

A pass rate with a threshold (and ideally a confidence interval) is the honest assertion. Track the
**flake rate** as a number - a suite that passes 80% of the time is telling you something. And keep
these slow, probabilistic tests *out* of the fast CI gate; run them on a schedule or a labelled
suite.

## Grade, don't compare

For a single run, what does "correct" even mean? Use the cheapest grader that will do:

* **exact match** for structured output, where there is one right answer;
* **a classifier** (Jev, from the classifier chapters) for labels and categories;
* **an LLM-as-judge** for prose, calibrated against a set of human-labelled examples;
* **a human** for the tail you can't automate.

Then gather a **golden set** of cases - and every production failure becomes a new case. That's how
the suite grows into something worth trusting, and it's what "evals" means in practice. The RAG
chapter builds one of these for a retrieval system, in
[Evaluating RAG Results](../04_rag/evaluating_rag.md).

## Design for the bad answer

Testing and design meet in one place: what does your system *do* when the model returns something
awful? Not "whether" - "what". The healthy pattern, which we built once already in the structured
output chapter:

1. **Validate** the answer against your rules.
2. **Retry** with the problem described back to the model - but **bounded**.
3. **Fall back** to something deterministic, or escalate to a person.

That's a loop guard wearing a test's clothing. The point is not to make the model perfect; it's to
make one bad answer a non-event.

> Don't chase 100%. You're not looking for a test suite that proves the model is right - you're
> looking for one that tells you, quickly and repeatably, when the *system* has changed.
