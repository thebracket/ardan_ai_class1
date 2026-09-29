# Ingesting a Manual

> **Draft — planning notes.**
> This will be in the repo as `ex26_vector_search` (provisional number).

We have embeddings and a distance search. Now let's point them at something real: my
[Ultimate Rust Foundations](https://github.com/thebracket/ArdanUltimateRust-5Days) class notes.

## Covered Here

* Where the corpus lives, and that we ingest prose only - not the `code/` directory.
* Walking the markdown files and cutting each one into chunks (the details live in
  [Chunking](./chunking.md)).
* Embedding every chunk with `fastembed`, the same model as `ex24_embeddings`.
* Storing chunks and vectors in SQLite, using `rusqlite` with the `bundled` feature so there is
  nothing to install.
* How a `Vec<f32>` becomes bytes: little-endian `BLOB`s, and back again.
* Storing the metadata we will want later: source path, heading, and eventually who may read it.
* Searching - embed the query, run the same cosine top-k as `ex24_embeddings`, print the hits with
  citations.
* Why we brute-force rather than index at this size, and when that stops being fine.
