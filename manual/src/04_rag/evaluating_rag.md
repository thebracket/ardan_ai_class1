# Evaluating RAG Results

> This is in the repo as `ex28_retrieval_eval`.

Here's the uncomfortable thing about retrieval: when it fails, nothing tells you. The bot gets the
wrong chunks, the model answers confidently from them anyway, and the user has no idea the answer
came from a page about something else. You cannot see this by trying one question by hand.

So before we make retrieval any cleverer, we build a scoreboard.

## The Golden Set

The scoreboard needs answers. Not complete ones - just: for this question, *which file should the
answer have come from*? In the example that's a small list in the source:

```rust
Question { text: "How do I find and fix a memory leak?", expected: "MemoryLeaks.md" },
Question { text: "What is async/await?",                 expected: "AsyncAwaitOverview.md" },
Question { text: "How do I stop threads from corrupting shared data?",
           expected: "Mutexes.md" },
```

The `expected` value is a path substring, so the same golden set works whether you point at the
bundled sample or the full notes. That's it - no labels, no scores, just "the answer lives here".

> An honest caveat: ten questions that *we* wrote is a tiny, biased set. Real golden sets are built
> from real user questions, are much larger, and get reviewed. But even this little one will change
> how you think about the rest of the chapter.

## The Contenders

We run three different searches over the corpus and score each one the same way:

* **keyword** - count how often the question's words appear. The naive version from `ex25_text_search`.
* **full-text** - SQLite's FTS5 index, ranked by BM25. The version you'd actually ship.
* **semantic** - embed the question, cosine against the chunks. From `ex26_vector_search`.

Each produces a ranking of files, and we score the position of the expected file:

* **hit@k** - the fraction of questions where the expected file landed in the top `k`.
* **MRR** (mean reciprocal rank) - average of `1 / rank`, so finding it first is worth `1.0` and
  finding it fifth is worth `0.2`. A miss scores zero.

We collapse each search down to one entry per *file* before scoring, so the semantic search doesn't
get an unfair advantage from having several chunks in the same document.

## The Bundled Sample: Too Easy

With six files, semantic search is perfect and the whole thing is a bit boring:

```text
Method        hit@1   hit@3   hit@5     MRR
keyword        0.10    0.70    0.90    0.39
full-text      0.80    0.90    1.00    0.88
semantic       1.00    1.00    1.00    1.00
```

Worth noticing even here: naive keyword counting is *bad*. Counting raw occurrences drowns in common
words like "do", "the" and "how", so it barely finds anything first. FTS5/BM25 fixes that by weighting
rare words more - which is why "full-text" is the real keyword baseline, not "keyword".

## The Real Notes: Ninety Files

Point `RUST_NOTES_DIR` at the full class notes and the picture gets interesting:

```text
Question                                            keyword  full-text  semantic
How do I share data between threads?                    #50         #5        #7
How do I send data from one thread to another?          #24         #2        #1
What are the performance costs of locking a mutex?      #39         #1        #1
How do I use Rc to avoid lifetime problems?             #12         #1        #1
Why does the borrow checker complain about retu...      #14         #5        #1
How do I find and fix a memory leak?                    #90         #1        #5
What is async/await?                                    #33         #9        #1
What is the difference between a sender and a r...      #16         #4        #1
How do I stop threads from corrupting shared data?      #51        #30        #5
What should I learn first when starting with Rust?      #19        #13        #5

Method        hit@1   hit@3   hit@5     MRR
keyword        0.00    0.00    0.00    0.04
full-text      0.30    0.40    0.70    0.44
semantic       0.60    0.60    0.90    0.67
```

Four things fall out of that table:

* **Naive keyword is a strawman.** Scoring `0.00` at hit@1 means it never found the right file
  first. Stop counting raw frequencies; use an index.
* **BM25 is a genuinely strong, cheap baseline.** `0.70` at hit@5 with no model, no downloads, and
  microseconds per query.
* **Semantic wins overall** (`0.60` vs `0.30` at hit@1) - and it earns it on the paraphrases. "How do
  I stop threads from corrupting shared data?" has almost no words in common with the answer, and
  semantic finds it at `#5` while full-text is down at `#30`.
* **But BM25 wins outright on some questions.** "How do I find and fix a memory leak?" is `#1` for
  full-text and `#5` for semantic. Exact, distinctive words are exactly where keyword search shines.

## What This Means

Neither search "wins". They fail in different places, which is the whole argument for keeping both -
and, later, for combining them. It also tells you what to do in practice: keyword search is a
perfectly good default that costs nothing, and semantic search is worth adding where the questions
are the kind people actually ask, phrased in their own words.

Two boundaries worth stating plainly:

* **This scores retrieval, not answers.** A perfect hit@1 can still produce a bad answer if the
  model misreads the passage. Retrieval quality is necessary, not sufficient.
* **The golden set is the hard part.** The measurements are easy once you have good questions and
  correct targets. Building that set - and keeping it honest - is the real work, and it's why
  "It's a Data Problem" comes later in this chapter.

Next we stop choosing between keyword and semantic, and run both.
