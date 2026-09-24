# Different Model Types

We've been using `deepseek/deepseek-v4.1-flash` throughout, with reasoning enabled so we can watch
the green thinking scroll past. That's a good default for a class, but it isn't the only choice,
and the choice matters more once you're building agents.

Roughly speaking, you can think of models on a spectrum:

* **Small, fast, cheap models.** Great at narrow, well-specified jobs: classify this text, extract
  these fields, decide between four labels, rewrite this sentence. Fast enough to sit inside a
  loop, cheap enough to call thousands of times.
* **Large reasoning models.** Slower and dearer, but much better at the messy, open-ended part:
  deciding what to do next, weighing evidence, recovering when a plan falls apart.

An agent doesn't have to use one model for everything. A common and very sensible shape is to use
a **small model for the mechanical steps and a big one for the decisions** - the same split we
talked about on the previous page, only now the "code" doing the deterministic work is itself a
small model. A cheap classifier can route a request; an expensive reasoner can handle the cases
the classifier can't resolve.

This is also a useful lens on cost and latency. Agents multiply model calls - every turn of the
loop is another request - so putting a small model in the hot path can be the difference between a
demo and a product. It cuts the other way too: a model that's too small will loop, hallucinate a
tool, or fail to notice it already has the answer, and then you've paid for ten cheap calls to
avoid one good one.

It's worth experimenting on the note-taking agent from earlier: run the same goal against a small
model and a reasoning model and compare the traces. Sometimes the small one is fine. Sometimes it
wanders in circles while the bigger one walks straight to the answer. Knowing which is which, for
*your* task, is the skill.
