# Full Text Search

> **Draft — planning notes.**
> Same example as [Keyword Search](./keyword_search.md) - `ex25_rag_search`.

## Covered Here

* The step up from scanning: the **inverted index** - a map from word to the documents it appears in.
* SQLite's FTS5 extension: `CREATE VIRTUAL TABLE ... USING fts5(...)`, inserting, and querying.
* Tokenisation and stemming: why `running` can match `run`, and when that's the wrong call.
* Ranking: BM25 in a sentence or two - rarer words count for more, and frequency has diminishing
  returns - and why *ordering* matters as much as matching.
* Phrase and boolean queries: `"exact phrase"`, `term AND term`, `term NOT term`.
* Snippets and highlighting, so a result can show its working.
* Where full-text search still can't help: synonyms and meaning (`cargo` versus `build system`).
* Why this is the bar the semantic search has to clear - and, honestly, often doesn't.
