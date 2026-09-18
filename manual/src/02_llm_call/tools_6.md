# Safety Options for Tools

So, like a lot of engineering - we're faced with a dilemma/trade-off. The more tools we grant to the harness, the more room there is for something bad to happen. But the fewer tools we give
to our harness, the less it can accomplish.

There isn't really a *perfect* answer here, and a lot depends upon what your organization needs. A coding harness (like opencode, claude code, codex, etc.) obviously has a LOT of freedom. It's
unavoidable, when the harness is writing code, testing it, and executing whatever code you've created. A customer service agent, however, really doesn't need to be able to access anything
beyond a very restricted list of appropriate items.

If your harness is only called by you, you're guarding against your mistakes (or an overzealous AI who "cleans up" your documents by deleting them). If your harness is called by your
coworkers, you are now trusting every coworker. If your harness is open to the Internet - good luck, the game just got a LOT harder, and you absolutely need to be *very* careful in what
it can do.

With the tool system, you can make the agent run *any code*.

## What Not To Do

Please, absolutely, never rely on a regex or string search for safety. Writing an `rm -rf` detector might be defeated by:

* `rm -fr`
* `find . -exec rm {}`
* `find . -delete`
* A program that calls the `unlink` syscall directly.

That's a losing battle from the start, so choose not to fight it.

Instead, you have four axes to attack:

* **Governance**. Track the identity of the *calling user*, and *every tool call* checks that the user is permitted to make the call.
* **Sandboxing**. This can happen at several levels:
  * *User Identity*. UNIX (and other operating systems) have pretty good user security built-in. Run your harness/agent as an unprivileged user, with only access to what they need.
  * *Containers*. Contain your harness/agent inside a container such as Docker, and be VERY restrictive as to what they can access.
  * *chroot*. It's an older form of sandbox, but it works on almost every \*NIX derived OS.
  * *Actual Sandboxes*. *Bubblewrap* is a popular sandbox; it uses the operating system's container system to keep a process from touching anything outside of the sandbox.
* **Limit Tools**. If your process just needs to enumerate users and statuses, don't give them `bash` and a users/statuses system. Give them a tool that *specifically* provides the data they need.
* **Embed Security Checks in Tools**. Especially for databases and APIs, pass the user identity to the tool and - if the db has security - use it. For example, if you have multiple tables of customer data and have group controls for who can access what; don't forget to use it!

> Note that OpenAI reports several sandbox escapes. These agents can be smart!

There's a curious thing here. This security list sounds a lot like something every experienced sysadmin/devops/developer already knows: if you have input from the outside, you can't trust it. At all.
