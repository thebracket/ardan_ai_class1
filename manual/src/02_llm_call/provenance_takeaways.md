# What Provenance Buys You

## Try It Live

Change the two constants at the top of `main.rs` and re-run:

```bash
cargo run -p ex15_provenance
```

| `DEFENCE_LEVEL` | `ATTACK` | What you should see |
| --- | --- | --- |
| 0 | `Pirate` | Full pirate |
| 1 | `Pirate` | Pirate, diluted to a `YARRR!` |
| 2 | `Pirate` | Clean weather report |
| 2 | `EmailAdapted` | `send_email` goes through — the envelope is defeated |
| 3 | `EmailAdapted` | `send_email` blocked by the gate |

## The Takeaway

* **Explicit provenance** is worth doing. It makes simple attacks fail and gives the model a fair chance. Treat it as defence in depth, not as the fence.
* **Enforced provenance** is the part that survives the model being fooled. It doesn't care how persuasive the injection is, because it never consults the model's judgement.
* The general rule — **the authority of an action must match the provenance of the text that prompted it** — is what you should carry away. Everything else on this page is an implementation of it.

## What's Still Missing

Our taint bit is deliberately simple: *any* tool output taints the session until the user speaks again. A real system needs more than that:

* **Per-source provenance**, not one global bit. A trusted internal database and a random web page are not the same thing.
* **A confirmation step** for privileged actions, rather than a flat refusal. "The model wants to send an email after reading a web page. Allow? [y/N]" is often better than "no".
* **An audit log** of what was blocked and why — so you can see attacks in the wild rather than guessing.
* **Least privilege**: scope each tool to the smallest capability that works, so that even a mistake can't do much.

Those last few are governance, and that is where we go next.
