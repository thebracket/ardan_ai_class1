# Evaluating RAG Results

> This is in the repo as `ex28_retrieval_eval`.

Here's the uncomfortable thing about retrieval: when it fails, nothing tells you. The bot gets the
wrong chunks, the model answers confidently from them anyway, and the user has no idea the answer
came from a page about something else. You cannot see this by trying one question by hand.

So before we make retrieval any cleverer, we build a scoreboard.

## The Golden Set

The scoreboard needs answers. Not complete ones - just: for this question, *which file should the
answer have come from*? In the example that's a small list in the source, and each entry carries the
question **twice**:

```rust
Question {
    text: "How do I find and fix a memory leak?",
    keywords: "memory leak",
    expected: "MemoryLeaks.md",
},
```

`text` is the question the way a person would ask it. `keywords` is the same need, phrased the way
someone who knows the vocabulary would type it. That second form matters enormously, and we'll come
back to it.

> An honest caveat: ten questions that *we* wrote is a tiny, biased set. Real golden sets are built
> from real user questions, are much larger, and get reviewed. But even this little one will change
> how you think about the rest of the chapter.

## The Contenders

We run three kinds of search, and score each one the same way:

* **keyword (count)** - count how often the question's words appear. The simplest possible thing,
  and a *bad* one - see below.
* **full-text** - SQLite's FTS5 index, ranked by BM25. This is what people actually mean by "keyword
  search", and it is genuinely good.
* **semantic** - embed the question, cosine against the chunks, from `ex26_vector_search`.

Crucially, we run full-text search **twice**: once with the conversational `text`, and once with the
`keywords`. If we only ran it on the sentences, we'd be measuring keyword search at its worst.

Each search produces a ranking of files, and we score the position of the expected file:

* **hit@k** - the fraction of questions where the expected file landed in the top `k`.
* **MRR** (mean reciprocal rank) - average of `1 / rank`, so finding it first is worth `1.0` and
  finding it fifth is worth `0.2`. A miss scores zero.

We collapse each search to one entry per *file* before scoring, so semantic search doesn't get an
unfair advantage from having several chunks in the same document.

## What "Keyword" Means Here

Look at the `keyword (count)` row in the results below: it's terrible. It is also *not* a fair
representation of keyword search. Raw frequency counting drowns in stopwords and filler, so it
stumbles over "how do I" and never recovers. That's a badly built baseline, and it earns its score.

**BM25 is the real thing.** It's the algorithm behind "full-text search" in every database you've
used, it weights rare and distinctive words far more heavily, and it's been tuned by decades of real
search traffic. When someone says "keyword search still wins", *this* is what they mean - and they're
usually right, especially when the person typing knows what to type.

## The Bundled Sample: Too Easy

With six files, everything that can work does work:

```text
Method            hit@1   hit@3   hit@5     MRR
keyword (count)    0.10    0.70    0.90    0.39
full-text (q)      0.80    0.90    1.00    0.88
full-text (kw)     1.00    1.00    1.00    1.00
semantic           1.00    1.00    1.00    1.00
```

Only one thing to note: even on six files, naive counting loses four questions outright. The rest is
a tie. We need a bigger corpus for this to mean anything.

## The Real Notes: Ninety Files

Point `RUST_NOTES_DIR` at the full class notes:

```text
Question                                       count   bm25 q  bm25 kw semantic
How do I share data between threads?             #50       #5       #2       #7
How do I send data from one thread to ano...     #24       #2       #1       #1
What are the performance costs of locking...     #39       #1       #2       #1
How do I use Rc to avoid lifetime problems?      #12       #1       #1       #1
Why does the borrow checker complain abou...     #14       #5       #1       #1
How do I find and fix a memory leak?             #90       #1       #1       #5
What is async/await?                             #33       #9       #9       #1
What is the difference between a sender a...     #16       #4       #6       #1
How do I stop threads from corrupting sha...     #51      #30       #2       #5
How do I create a mutable variable in Rust?      #23       #1       #1       #1

Method            hit@1   hit@3   hit@5     MRR
keyword (count)    0.00    0.00    0.00    0.04
full-text (q)      0.40    0.50    0.80    0.53
full-text (kw)     0.50    0.80    0.80    0.68
semantic           0.70    0.70    0.90    0.75
```

Now there's something to argue about. Read it carefully, because the headline "semantic wins" is not
the interesting part.

* **Keyword search, queried properly, is excellent.** BM25 on the conversational sentences manages
  `0.40` at hit@1. Change nothing but how we phrase the query - `mutex performance` instead of "What
  are the performance costs of locking a mutex?" - and it jumps to `0.50` at hit@1 and **`0.80` at
  hit@3**. That's better than semantic search at hit@3, with no model, no downloads, and a
  microsecond query time.
* **Query style is a hidden variable.** If we'd only tested conversational questions, we'd have
  concluded embeddings were clearly better. Half of that gap was just us asking in a way keyword
  search likes. Anyone who grew up picking search terms - the Google-era skill - already has this
  superpower; a good keyword search rewards it.
* **Semantic search wins overall, but modestly.** `0.70` vs `0.50` at hit@1 and `0.90` vs `0.80` at
  hit@5. It earns its keep on the paraphrases: "How do I stop threads from corrupting shared data?"
  has almost no words in common with the answer, and embeddings find it at `#5` while BM25-on-the-
  sentence languishes at `#30`.
* **Neither dominates.** BM25 wins outright on "memory leak" (`#1` vs `#5`). Semantic wins on the
  corrupted-data question. That's the whole argument for combining them, which is next.
* **Naive counting is a trap, not a technique.** `0.00` everywhere. If your "keyword search" is a
  `contains` loop, you haven't measured keyword search; you've measured a bad loop.

## What This Means

The practical lesson isn't "use embeddings". It's:

* Keyword/full-text search is a *strong default*. It's cheap, instant, explainable - you can see why
  a result matched - and with a competent query it beats a local embedding model on a third of the
  questions here.
* Semantic search is worth adding where your users type prose rather than terms, and it's worth
  measuring rather than assuming.
* And the only way to know which is winning *on your data, with your users' queries* is a scoreboard
  exactly like this one. The numbers here are ours; yours will differ, and that's the point.

Two boundaries worth stating plainly:

* **This scores retrieval, not answers.** A perfect hit@1 can still produce a bad answer if the model
  misreads the passage. Retrieval quality is necessary, not sufficient.
* **The golden set is the hard part.** The measurements are easy once you have good questions and
  correct targets. Building that set - and keeping it honest - is the real work, and it's why "It's a
  Data Problem" comes later in this chapter.

Next we stop choosing between keyword and semantic, and run both.
