# Governance

In "Safety Options for Tools" we met governance as one of four axes: **track the identity of the
calling user, and check every tool call against what that user is allowed to do.** This page is
about that axis - because it's the one people reach for a product to solve, and because it gets
conflated with two other things.

"Governance" means at least three different things, and they ship as different products:

* **Authorization.** Is *this* call, by *this* identity, allowed? A policy decision.
* **Guardrails.** Is this content or action safe - prompt injection, PII, something abusive?
* **Compliance (GRC).** Can we show an auditor that we followed the policy?

They overlap, but they are not the same, and buying one does not get you the other two. For a
tool-calling harness the one that matters most is the first: a model will request a tool call, and
*your code* decides whether to honour it.

The shape of that check is small:

```rust
/// Every tool call is made on behalf of somebody. Pass that identity all the
/// way down - and never lose it at the model boundary.
fn handle_tool_call(user: &User, tool: &str, args: &Value) -> Result<String> {
    // Default deny. If policy doesn't explicitly allow it, it doesn't happen.
    if !policy::allows(user, tool, args) {
        audit::record_denied(user, tool, args);
        return Err(anyhow!("not permitted"));
    }
    let result = dispatch(tool, args);
    audit::record_allowed(user, tool, args);
    result
}
```

Three habits matter more than the mechanism:

1. **Carry the identity, don't infer it.** The user driving the harness is not the model. Thread
   their identity down to the tool call, and never let tool output change who is asking.
2. **Default deny.** Allow by explicit rule, not by the absence of one.
3. **Audit every decision**, allowed or denied. A decision you can't reconstruct later is a
   decision you can't defend.

Notice this is the same shape as the provenance checks from the injection chapters - *know who is
asking, and know where the data came from* - only now the question is permission rather than trust.
In [Governance in the Real World](../07_governance/in_the_real_world.md) we'll look at the systems
people buy to run this at scale.
