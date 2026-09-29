# Cheap Tags: Keywords from Small Models

> **Draft — planning notes.**
> Tentatively `ex28_rag_keywords` (provisional number).

Keyword search is cheap and often wins. So what if we make the documents carry *more* and *better*
keywords than they happen to contain? That is a job a small model can do well - once per chunk, at
ingest time.

## Covered Here

* The shape of the trick: at ingest, ask a cheap model for the handful of keywords or tags that
  describe each chunk, and store them next to it.
* Why it can pay off: query-time matching stays instant and free, and the model adds the words a
  reader would have used but the document didn't - `concurrency` on a page that only says "threads".
* Prompting for it: "list up to five keywords" is the low-tech version; asking for a JSON array is the
  slightly better one.
* Structured output: reusing the typed-output idea from `ex16_structured` and the helper-model pattern
  from `ex21_jev`, so keywords arrive as data rather than prose to parse.
* Controlled vocabularies: give the model a fixed list of tags to choose from instead of inventing
  them, and why that keeps the index usable.
* Where to spend the calls: one cheap call per chunk at ingest, versus zero at query time - and what
  that means for cost, and for staleness when a document changes.
* The honest bit: uncontrolled tags drift, overlap, and multiply. Without a curated vocabulary you get
  a tag cloud, not an index.
* The question worth answering on stage: do model-generated keywords beat plain full-text search, or
  just cost more? (This is another job for the scoreboard.)
