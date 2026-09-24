# Workflows

Everything in this section has been circling the same fork in the road. On one side, an **agent**:
you give it a goal and it decides the path. On the other, a **workflow**: you decide the path ahead
of time, and let the model - if any - do only the parts you've chosen.

A workflow is a fixed sequence or graph of steps that you wrote down. The control flow lives in
your code (or your config), not in a model's head. Steps can absolutely *use* a model - summarise
this, draft a reply, classify that ticket - but which step runs next was decided before you ever
called one.

| | Agent | Workflow |
| --- | --- | --- |
| Who chooses the path | The model | You |
| Determinism | No | Yes |
| Testing | Hard | Like any other code |
| Flexibility | High | Only the paths you wrote |
| Failure mode | Wanders, loops, surprises you | A branch you forgot to handle |

The appeal is the boring stuff you should care about: workflows are cheaper, faster, testable,
observable, and resumable. If a run dies halfway through, you know exactly where it was, because
*you* put the steps in order.

The cost is that a workflow only knows the cases you anticipated. The trick is realising that
"decide which workflow applies" is itself a step - and often a small, solvable one. We'll look at
defining them, and then at choosing between them.

> The smallest possible workflow is the `no_agent` cron job from the always-on chapter: a script
> runs, its output is the message, and no model is involved at all. Start there and add a model only
> when a step genuinely needs one.
