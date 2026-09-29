# Audit

[Observability](../06_production/observability.md) and audit look similar - both are records of what
happened - but they have **opposite guarantees**, and that difference is the whole point.

| | Observability | Audit |
| --- | --- | --- |
| Audience | Engineers | Compliance, security, auditors |
| Completeness | Sampled, lossy is fine | Complete - nothing dropped |
| Mutability | Can be rewritten/scrapped | **Immutable**, append-only |
| Retention | Days to weeks | Per policy, often years |
| Access | Broad | Tightly controlled |
| Question | "Why was it slow?" | "Prove who did what, when" |

If you sample your audit log, you don't have an audit log. If you keep every prompt forever, you
have a privacy liability. Treat them as separate stores.

## What a record has to contain

For a tool-calling system, the interesting unit isn't the HTTP request - it's the **decision**.
A defensible audit record says:

* **Who** - the identity driving the harness (not the model);
* **What** - the tool, its arguments, and the model call that led to it;
* **When** - a trustworthy timestamp;
* **With what** - the **model version**, **prompt version**, and **tool version**. Without these,
  the record isn't reproducible, and an audit you can't reproduce isn't one;
* **The decision** - allowed or denied, and under which policy;
* **Provenance** - which inputs were trusted, and which came from untrusted tool output.

That last pair is where this meets the rest of the course: it's the governance check and the
provenance rules, written down.

## The properties that make it an audit

* **Complete.** Not sampled, not best-effort. If the process can act, it must record.
* **Append-only and tamper-evident.** Write once, and make silent edits detectable (an append-only
  store, a hash chain, or a ledger).
* **Retained** for a defined period, and **access-controlled** - the audit store is itself a target.
* **Correlated** to the observability trace, so an engineer can follow a number on a graph to the
  full record without the two sharing a mutability model.

## The bits people forget

* **Record denials and changes, not just actions.** "Nobody was allowed to do X" is as important as
  the actions that happened. Policy changes need an author and a timestamp too.
* **Don't put the payload in the audit if you don't have to.** Log identifiers and hashes where
  possible; put the sensitive content in the access-controlled store, or redact it.
* **Reconcile audit with erasure.** "Right to be forgotten" and "keep everything for seven years"
  pull in opposite directions; decide the policy up front, and store references rather than raw
  personal data where the law requires it.
* **Audit the auditor.** Who read the audit log, and when?

> The test is simple: if you can't reconstruct a decision months later - who asked, what ran, which
> versions, under which policy - then you don't have audit. You have logs, and logs are not a
> defence.
