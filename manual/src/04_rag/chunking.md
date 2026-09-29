# Chunking

> **Draft — planning notes.**
> Same example as [Ingesting a Manual](./ingesting_a_manual.md) - `ex26_vector_search`.

## Covered Here

* Why we can't embed whole documents: context limits, and one vector can't stand in for a whole page.
* Why we can't embed single sentences either: too little context to mean much.
* The middle ground - split on markdown headings, so each chunk is roughly one idea.
* Keeping a heading path (`Mutexes > Performance of Mutexes`) attached to the chunk, because a chunk
  torn out of context often loses the thing that made it findable.
* What to do with fenced code blocks: keep them with their prose, or strip them.
* A small overlap between chunks so a sentence doesn't get cut in half.
* The before/after demo: the same question, retrieved with bad chunks and with good ones.
* The honest lesson: bad chunking is the most common cause of bad RAG, and no reranker fixes it.
