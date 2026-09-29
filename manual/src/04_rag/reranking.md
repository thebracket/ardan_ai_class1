# A Second Opinion: Rerankers

> This is in the repo as `ex30_rerank`.

Both searches we've built so far score the query and the document more or less *separately*. BM25
looks at which words overlap; the embedding model turns each side into a vector on its own and then
compares the vectors. Neither one ever sits down and **reads the pair together**.

A **reranker** (a cross-encoder) does exactly that. You hand it the query and one candidate passage
in the same input, and it returns a single relevance score. Because it sees them at the same time, it
can tell that "How do I stop threads corrupting shared data?" is closely answered by a passage about
mutexes even when they share almost no words.

## Bi-Encoder vs Cross-Encoder

This is worth naming, because it explains the cost:

* The embedding model is a **bi-encoder**: it encodes the query and each document independently, so
  you can pre-compute and cache every document vector and compare with a dot product. That's what
  makes semantic search fast enough to run over thousands of chunks.
* The reranker is a **cross-encoder**: it encodes query-plus-passage in one pass. Nothing can be
  pre-computed - the score only exists for *this* pairing - so running it over the whole corpus for
  every query would be hopelessly slow.

That's why you never rerank everything. You use it as a **second stage**: retrieve a shortlist cheaply
with the hybrid search, then let the cross-encoder reorder just those candidates.

## The Pipeline

```text
question ─▶ hybrid search (BM25 + embeddings, RRF) ─▶ top 50 candidate passages
                                                          │
                                                          ▼
                                            cross-encoder scores each one
                                                          │
                                                          ▼
                                                    reordered top-k
```

In the code, stage one is what we built last time, and stage two is a few lines:

```rust
let mut reranker = TextRerank::try_new(
    RerankInitOptions::new(RerankerModel::BGERerankerBase)
        .with_cache_dir(cache_dir())
        .with_show_download_progress(true),
)?;

// `documents` is the candidate passages, in retrieval order.
let scores = reranker.rerank(question, documents, false, None)?;
// `scores` comes back sorted best-first; each result carries the `.index`
// of the candidate it scored, so you can map the new order back.
```

We feed the reranker the heading and the body, but **not** the embedding model's `passage: ` prefix -
that prefix is an instruction to E5, not part of the document, and handing it to a cross-encoder just
adds noise. Small detail, easy to get wrong.

## The Result - and the Honest Bit

Same golden set, same scoreboard, on the full notes:

```text
Method        hit@1   hit@3   hit@5     MRR
semantic       0.70    0.70    0.90    0.75
hybrid         0.60    1.00    1.00    0.78
rerank         0.70    0.80    0.90    0.79
```

This is not the clean "rerank wins everywhere" story you might expect. Read it carefully:

* **Rerank has the best hit@1 and the best MRR.** It is genuinely better at putting the right answer
  *first* - `0.70` vs hybrid's `0.60`, and the best MRR of the three. The cross-encoder's whole job is
  sharpening the top of the list, and here it does.
* **Rerank is worse at hit@3 and hit@5.** It drops from hybrid's perfect `1.00` to `0.80` and `0.90`.
  Reordering the shortlist means some correct answers get pushed *down* as well as up. Look at the
  question about threads corrupting shared data: hybrid had it at `#2`, the reranker moved it to `#9`.

So the lesson is precision versus recall, and it's a real trade-off, not a bug:

* A reranker **reorders** candidates; it can never recover an answer the first stage didn't retrieve.
  This is why the candidate count matters - too few, and no reranker can save you.
* It improves *precision at the very top*, which is what you want when you're going to hand one or two
  passages to a model. It does not always improve recall.
* Whether it helps **on your data** depends on the model, the chunk sizes, and what "relevant" means.
  Which is, once again, an argument for the scoreboard.

## Model Choice Is Not Neutral

We ran this with `jinaai/jina-reranker-v1-turbo-en` first - small, fast, and reasonably good - and it
scored worse than hybrid outright (`0.50` hit@1, `0.69` MRR). Swapping to `BAAI/bge-reranker-base`
gave the numbers above. Same pipeline, same candidates, same queries; only the model changed. Rerankers
are not interchangeable, and this one is a real download (hundreds of megabytes), unlike the tiny
embedding model.

## A Worked Example

For "What is async/await?", retrieval's fused order put the right file third:

```text
retrieval:  03-Async/HelloAsync.md > Hello Async/Await,
            03-Async/TokioFutures.md > Tokio Futures > Await,
            03-Async/AsyncAwaitOverview.md > Async/Await Overview > Rust and Async/Await, ...

reranked:   03-Async/AsyncAwaitOverview.md > Async/Await Overview,
            03-Async/AsyncAwaitOverview.md > Async/Await Overview > Rust and Async/Await,
            03-Async/HelloAsync.md > Hello Async/Await > What's Actually Happening Here?, ...
```

The reranker pulled the right file's two best chunks to the top. That's the behaviour you're buying -
and the other questions are where you find out what it costs you.

## Where This Leaves Us

Retrieval, in full, is now: keyword search, semantic search, fuse them, rerank the shortlist. That's a
real, production-shaped pipeline, and every stage earned its place on a scoreboard rather than a hunch.
Everything from here is about the *data* going into it - chunking it well, tagging it cheaply, and
keeping it safe - which is where the rest of this chapter goes.
