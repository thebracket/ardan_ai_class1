# Let the Bot Read the Docs

> This is in the repo as `ex27_rag_chat`.

Here it is - the actual **generation** half of Retrieval Augmented Generation. Everything so far has
been retrieval: finding the right passages. Now we hand those passages to a model and let it answer
from them. This is the first example in the chapter that genuinely earns the name RAG.

## The Tool

We wrap retrieval in a tool, exactly the way we wrapped `get_time` and `roll_dice` back in
[Generic Tool Calls](../02_llm_call/tools_3.md). The model decides *when* to search; the tool does the
search and hands back the text:

```rust
fn tools(&self) -> Vec<ToolDefinition> {
    vec![ToolDefinition {
        name: "read_docs".to_string(),
        description: "Search the Ultimate Rust Foundations class notes and return the most \
                      relevant passages, each with its source file and heading."
            .to_string(),
        parameters: vec![("query".to_string(), "string".to_string())],
        required: vec!["query".to_string()],
    }]
}
```

When the model calls `read_docs`, we embed the query, run the same cosine top-k from the last few
pages, and format the winners for the model to read - each one labelled with its path, heading and
score, so it can cite what it used.

## Build Once, Then Chat

The corpus is embedded **once, at startup**. The program says so, and then it says nothing else until
it's ready:

```text
Building corpus...
Corpus ready: 18 chunks from 6 files
Ask a question about the notes, or /quit to quit
```

Doing the slow part up front is what makes the rest bearable: by the time the user types anything, every
chunk has a vector sitting in memory, and a search is a few hundred multiplications.

> A note on `async`: `fastembed` is synchronous, and we call it directly from `async main` - including
> the query embedding inside the tool. Strictly, that blocks the executor for the moment it's running.
> For a teaching example, where there is one user and one query at a time, that is completely fine, and
> reaching for `spawn_blocking` here would add machinery that obscures the point. In a server, with
> concurrent users, move the model to a blocking worker and keep the async reactor clear.

## The System Prompt

The prompt is where RAG earns its keep, and it's short:

> You are a helpful assistant for the Ultimate Rust Foundations class notes. Use the `read_docs` tool to
> look things up before you answer a question about the notes. Answer using only the passages the tool
> returns, and name the source file and heading you used. If the passages do not contain the answer, say
> so rather than guessing.

Three instructions do the work: *search first*, *answer only from what you found*, and *say when you
didn't find it*. The last one is the hardest for a model to obey - models would rather help than admit
ignorance - and it's the one that stops RAG from becoming a confident bullshitter.

## What Happens When You Ask

```text
Ask a question about the notes, or /quit to quit
How do I share data between threads?
Let me search the docs.🔧 Tool call: read_docs
```

Then the model streams back a cited answer. Asked about `Arc`, it pulled the passages, cited
`04-Memory/Lifetimes.md > Combine with Rc`, and added - unprompted - that the notes only mention `Arc`
in passing. That's the behaviour you want: it used the retrieval, it showed its sources, and it was
honest about the edge of what the corpus contains.

## The Honest Bits

* **The model might not call the tool.** Nothing forces it. If the system prompt is weak or the
  question looks trivial, it may just answer from memory - and then you have no idea whether it read
  your docs or hallucinated. Prompt, and check the tool-call log.
* **The notes reference code the model cannot see.** The prose is full of "the code for this is in the
  `code/02_threads` folder", but we only ingested the prose. So the model will happily *invent* the
  code it can't read. This is a wonderful live demo of a model reasoning past the edge of its context,
  and a reminder that RAG answers from documents, not from the world.
* **Retrieval quality is the ceiling.** If the right chunk wasn't retrieved, the model either guesses
  or, if you followed the system prompt, tells the user it couldn't find the answer. Both are better
  than confident nonsense, and neither is as good as fixing the retrieval.

And with a bot now reading untrusted documents and answering from them, we've built something worth
worrying about - which is where the rest of the chapter goes.
