# Full Text Search

> Same example as [Keyword Search](./keyword_search.md) - `ex25_text_search`.

Scoring by raw occurrence count has an obvious flaw: every word counts the same, so `the` is worth as
much as `mutex`. The fix is older than the web, and SQLite gives it to us for free.

## The Inverted Index

Instead of scanning the text every time, build a map from each word to the documents that contain it -
an **inverted index**. Searching becomes "look up the word, read off the documents", and the engine can
afford to be clever about *ranking* while it's there.

SQLite's **FTS5** extension does the whole job. You declare a virtual table and query it with `MATCH`:

```sql
CREATE VIRTUAL TABLE docs USING fts5(path, body);

SELECT path
FROM docs
WHERE docs MATCH ?1
ORDER BY bm25(docs);
```

That's it. `build_full_text_index` in the example is those two lines plus an insert loop.

## BM25, In One Paragraph

The ranking function is **BM25**, and its intuition is easy. A word matters more when it is **rare** -
matching `mutex` is strong evidence, matching `the` is not - and appearing five times is better than
appearing once, but the tenth mention adds very little over the ninth. That's basically it: term
frequency, softened, weighted by scarcity. Decades of search engines were built on this, and it still
punches well above its weight.

## Seeing the Difference

Running both searches over the full notes for `mutex` shows keyword counting and BM25 agreeing on the
*order* but not on the *margin*:

```text
Keyword search (top 5):          Full text search (top 5):
    44  03-Async/SharedState.md      4.58  03-Async/SharedState.md
    33  02-SystemThreads/Mutexes.md  4.56  02-SystemThreads/Mutexes.md
    23  02-SystemThreads/Deadlocks.md 4.43 02-SystemThreads/Deadlocks.md
    19  04-Memory/ReferenceCounting.md 4.30 04-Memory/ReferenceCounting.md
     8  02-SystemThreads/WorkQueue.md  4.09 02-SystemThreads/WorkQueue.md
```

Raw counting says `SharedState.md` is *eight times* hotter than the bottom of the list; BM25 rightly
says these pages are all much of a muchness on the word `mutex`. That difference matters the moment a
query has more than one word in it - BM25 can tell that the *rare* one is the one you meant.

FTS5 also understands queries, not just words: `"exact phrase"` for adjacency, `term AND term`,
`term NOT term`, and `snippet()` / `highlight()` to show the matched text back to the user. All of that
comes free with the extension.

## Where It Still Fails

Full-text search fixes the *counting*, not the *meaning*. It cannot help you with:

* **Synonyms and paraphrase.** `cargo` versus `build system`; `drops` versus `destructor`. The words
  are different, so the match is zero.
* **Intent.** Ask a question in prose and BM25 does its best with the words it can see.
* **Word order and negation, at scale.** `NOT` works, but "how do I *avoid* deadlocks" is not a boolean
  query the user typed.

Which is the whole reason semantic search exists - and why the
[scoreboard](./evaluating_rag.md) had to compare them rather than assume. But make no mistake about the
order of operations: **BM25 is the real keyword baseline.** It is cheap, instant, explainable, and it
beats a local embedding model on a meaningful share of real questions. Reach for embeddings *in
addition to* this, not instead of it.
