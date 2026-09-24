# Be Deterministic When Possible

Agents are wonderful and terrible in roughly equal measure. Before you build one, it's worth
asking a slightly rude question: *could ordinary code do this?*

Because an agent is:

* **Non-deterministic.** Ask the same question twice, get a different trace - different tools,
  different order, occasionally a different answer.
* **Slow and expensive.** Every step is another round trip to a model, and you pay for the lot.
* **Hard to test.** You can't assert that a tool was called, or that the answer was reached a
  particular way, without a lot of scaffolding.
* **Hard to debug.** When it goes wrong, you're reading a transcript and guessing.

None of that is an argument against agents. It's an argument for using them *on purpose*. If you
already know the steps - fetch the user, look up their plan, apply the discount, send the email -
then write that down as code. It will be faster, cheaper, testable, and it will do the same thing
every time. A fixed sequence of steps is a **workflow**, and a workflow is usually just a function.

The interesting rule of thumb is: **use an agent for the decisions, use code for the steps.** Let
the model figure out *what* needs doing; let your program do the doing. That's the same "keep the
decisions in code" instinct we met when we returned structured data from tools instead of prose.

We'll come back to workflows properly later on. For now, the point is just to build the reflex:
before reaching for an agent, check whether a `match` and a couple of function calls would have
solved it. Often they will - and the next page is the cautionary tale.
