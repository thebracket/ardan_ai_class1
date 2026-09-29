# Keyword Search

> **Draft — planning notes.**
> This is in the repo as `ex25_text_search`.

Before embeddings, before indexes: the oldest search there is - look for the word.

## Covered Here

* The simplest possible search: does this document contain this word? `str::contains`, and its
  variants.
* Multiple terms, and the first crude ranking you get for free - how many terms hit, or how often.
* Why exactness is a feature, not a limitation: identifiers, error codes, `Arc<Mutex<T>>`, someone's
  name. An embedding model is often *worse* at these than a literal match.
* Where it falls over: synonyms (`run`/`sprint`), morphology (`run`/`running`), other languages,
  typos, and the difference between matching a word and matching part of a word.
* Case folding and Unicode, and why "just lowercase everything" is a start and not an answer.
* Cost: a full scan is `O(n)` per query, but a very cheap `O(n)`.
* The framing for the rest of the chapter: keyword search is the baseline to beat, and the scoreboard
  we build later is how we find out whether embeddings actually beat it.
