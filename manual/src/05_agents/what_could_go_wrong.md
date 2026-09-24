# What Could Go Wrong?

So far we've been cheerfully giving a language model a goal, a set of tools, and permission to
keep going until it decides it's finished. That's an enormous amount of trust to hand to something
that is, at heart, a very confident autocomplete. This section is about the ways that goes wrong -
and about what to do instead.

Most agent failures are not model failures. They're harness failures: we didn't cap the loop, we
didn't validate the output, we didn't check who was asking, we didn't notice it was running
unattended. The model is doing what it always does - predicting a plausible next token. The harness
is what decides whether that's allowed to cause a problem.

The usual suspects:

* **The loop never ends.** No iteration cap, or a stop condition the model never hits. It burns
  tokens and time and learns nothing. (We capped `ex20_agent` for exactly this reason.)
* **The loop does something destructive.** We covered tools and sandboxes earlier, but an agent
  with autonomy gets to point them wherever it likes. The prompt injection and provenance chapters
  are not background reading once the thing is running on its own.
* **Non-determinism.** The same goal takes a different path every time. Great for demos, awful for
  debugging, testing, or explaining to an auditor.
* **It's always on.** The moment your agent is on a timer - a cron job, a heartbeat - it's acting
  unattended, repeatedly, forever. We'll look at that next.
* **Structured output isn't reliable.** Ask for JSON and you *usually* get JSON. "Usually" is not a
  contract. That one deserves its own discussion.

None of these are reasons not to build agents. They're reasons to build them on purpose, with the
same discipline you'd apply to any other unattended process.

Next: the always-on agent, and why it's less exotic than it looks.
