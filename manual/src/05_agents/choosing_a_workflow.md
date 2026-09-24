# Choosing a Workflow

If you have several workflows, something has to pick one. That something is a **router**, and
here's the nice part: routing is a classification problem, and we already know how to do those
without reaching for an agent.

The shortlist, roughly in order of preference:

* **A lookup.** If the request carries a type, a form field, a ticket queue, or an obvious keyword,
  match on it. Deterministic, instant, free.
* **A classifier.** For messy natural-language input, a classifier (Jev, or a small local model) is
  the right shape: text in, one of *N* known workflows out. Compare it against the discussion a
  couple of pages back in "Classify, Don't Generate".
* **A small language model.** If the categories are fuzzy and you don't have training data, a cheap
  model with a tight prompt will do - just validate its answer against your list, and have a
  fallback.

What you *don't* do is ask the agent to "figure out what kind of request this is and then do the
right thing." That's just an agent with extra steps, and you've lost the determinism you were
reaching for.

Two habits make routing safe. Always have an **`Unknown` / `Other`** workflow, so there's a defined
path for the cases you didn't anticipate - one that might escalate to a human, or, deliberately,
hand off to an agent. And **log which workflow was chosen**, with the input that chose it, so you
can debug and improve the router.

> Notice how this loops back to the start of the section. "That Could Have Been An If Statement" was
> the warning; a well-chosen workflow is the fix. The agent hasn't gone away - it's now one branch
> among several, used exactly where its flexibility earns its keep.
