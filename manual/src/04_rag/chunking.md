# Chunking

> Same example as [Ingesting a Manual](./ingesting_a_manual.md) - `ex26_vector_search`.

If there's one page in this chapter that changes how you build RAG systems, it's this one. Retrieval is
only as good as the pieces it gets to retrieve from, and chunking decides what those pieces are.

## Why Not Whole Documents?

Because a vector can't hold a whole document. One `384`-number point cannot represent a page that talks
about mutexes, channels, deadlocks and poisoning; it ends up somewhere between all of them, which means
it's close to all of them and useful for none. There's also a hard limit: the embedding model truncates
its input (roughly `512` tokens for our E5 model), so a long page would be embedded by its first few
paragraphs and silently lose the rest.

## Why Not Sentences?

At the other extreme you get fragments with no context. "It is fast." is a sentence; as a chunk it is
almost meaningless, because the word "it" doesn't say what. You also end up with an enormous number of
tiny vectors, and a search that returns five nearly-identical sentence fragments rather than the
passage you wanted.

## The Middle Ground: Split on Headings

The natural size is a *section* - one idea, with its supporting text. Markdown hands us exactly that
structure for free, so the example splits on `#`, `##`, `###`, and stops at the next heading of the
same or higher level.

Two details make it work:

* **Track code fences.** A line starting with `#` inside a fenced code block is a comment, not a
  heading. The chunker flips an `in_code_fence` flag on every ``` and ignores headings while it's set.
* **Keep the heading path.** A chunk doesn't just remember "Performance of Mutexes"; it remembers
  `Sharing Data with Mutexes > Performance of Mutexes`. That trail is prepended to the text before
  embedding, because a section torn out of its parent often loses the words that made it findable.
  "Performance" alone is vague; "Mutexes > Performance" is a topic.

## The Knobs We Left Alone

Chunking has more dials than we turned, and each is a worthwhile experiment:

* **Code blocks.** We keep them, because they're part of the prose's meaning and we're embedding the
  notes wholesale. Stripping them is defensible - the page text gets denser - and it's a one-line
  change if you want to compare.
* **Overlap.** We cut cleanly at headings, so a thought that spans a boundary can lose its other half.
  Real systems often overlap chunks by a sentence or two. We don't, and you can feel it on the edges.
* **Size limits.** Even a single heading section can exceed `512` tokens. The embedding model just
  truncates, which is a quiet way to lose the end of a section.

## The Before-and-After

Here's a failure you can watch happen. On the full notes, a broad question like *"several threads all
trying to update the same list without stepping on each other"* pulls a **table-of-contents page** into
the results - often near the top. Look at what that chunk actually is: `README.md > Fearless System
Thread Concurrency > Second Hour`, which is nothing but a bulleted list of links - `ScopedThreads`,
`Atomics`, `Mutexes`, and so on.

That chunk mentions *every* threading topic, so it looks relevant to *every* threading query. It's the
worst kind of chunk: semantically it's near everything, which means it's near nothing useful. The fix
isn't a better model. It's to notice that navigation pages are not content - drop them at ingest, or tag
them and filter them out, or chunk them so they never compete with real sections:

```text
Bad chunk:   a page of links to threading topics      → matches every threading query
Good chunks: "Sharing Data with Mutexes > Performance" → matches the locking question
             "Sending Data Between Threads"            → matches the channels question
```

That's the whole lesson in miniature. The *same corpus*, the *same model*, the *same query* - and the
top result swings entirely on how the text was cut up.

## The Takeaway

Chunking is the highest-leverage knob in the entire pipeline, and it's the one people touch last. A
reranker reranks the candidates you gave it; it cannot rescue a chunk that split the answer in half or a
navigation page that outranks everything. Get the chunks right first, and the rest of the machinery has
something to work with.
