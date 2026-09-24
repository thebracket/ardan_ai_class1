# A Server You Shouldn't Trust

Everything we said in **MCP Security and Provenance** was about consuming somebody else's server.
Now we're the ones writing one, and the lesson doesn't get kinder - it cuts both ways.

## As a server author

You are about to hand a model a list of capabilities. Do it as if the model were a clever, eager
intern with no judgment about consequences:

* **Least privilege.** Expose the narrowest tools that do the job. A "manage the database" tool is
  a mistake; "look up one order by id" is not.
* **Validate everything.** The arguments come from a language model, which means from text that
  could have been influenced by anything upstream. Treat them like user input from the open
  internet: check types, check ranges, check that a path stays inside its fence.
* **Never trust a path.** If your tool takes a filename, resolve it and confirm it's inside the
  directory you meant. This is the same "don't write an `rm -rf` detector" lesson from the tools
  chapters, and it applies to every server you publish.
* **Don't put secrets in descriptions.** Tool descriptions are sent to the model and can end up in
  logs. So are error messages. Don't echo keys, tokens, or PII back in either.
* **Say who you are.** A clear server name and honest descriptions help the model (and the user)
  understand what's being offered.

## As a consumer

The other half is the one we already rehearsed: a server's tool descriptions are attacker-
controlled text that goes straight into the model's context. The provenance and prompt-injection
chapters are not theoretical. The fact that *you* wrote a trustworthy server doesn't make the
next one trustworthy, and a harness that can't tell them apart needs the safeguards to be in the
harness.

There's a nice symmetry in that. On the client side we built the machinery to keep untrusted tool
output from driving privileged actions. On the server side we're the ones producing that output -
so the standards we hoped other people would meet are now ours to meet first.

> If your server needs to touch the filesystem or run commands, go back to **Safety Options for
> Tools**. The four axes - governance, sandboxing, limiting tools, and checks inside the tool -
> apply to a server exactly as they did to a hand-written tool. Running it in a container isn't
> just for the workshop; it's how you ship it.
