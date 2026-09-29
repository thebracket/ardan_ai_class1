# Ingesting a Manual

> This is in the repo as `ex26_vector_search`.

We have embeddings and a way to measure closeness. Now let's point it at something big enough to be
interesting: a whole manual.

## The Corpus

The example reads my [Ultimate Rust Foundations](https://github.com/thebracket/ArdanUltimateRust-5Days)
class notes. There's a small shared sample in `code/rag_corpus/`, so it runs with no setup; point
`RUST_NOTES_DIR` at a full checkout and it reads all 90 files.

One decision matters before anything else: **we ingest the prose, not the code.** The walker skips
`code/`, `images/`, and hidden directories, and only collects `.md` files. That keeps the index focused
on explanations rather than source, and it's the safer default - the code directory is also where keys
and secrets tend to live.

## From Files to Chunks

`load_documents` walks the tree and reads each file. `chunk_markdown` then cuts each file into
sections - that's the next page's whole subject, but the shape is simple: split at headings, keep the
trail of headings that led there. On the sample that's `6 files → 18 chunks`; on the full notes,
`90 files → 398 chunks`.

## Embedding

Every chunk becomes a `384`-number vector with `fastembed` and `multilingual-e5-small` - the same local
model from [Searching by Meaning](./searching_by_meaning.md). The one detail worth repeating: documents
get the `passage: ` prefix and the *query* gets `query: `. E5 was trained that way, and getting them
the wrong way round quietly costs you quality.

## Storing Vectors in SQLite

Here's the part that surprises people: a vector is just bytes, and SQLite stores bytes perfectly well.
One table holds the text and its vector side by side:

```sql
CREATE TABLE chunks (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    heading TEXT NOT NULL,
    body TEXT NOT NULL,
    embedding BLOB NOT NULL
);
```

An `f32` is four bytes, so a `384`-dimension vector is 1,536 bytes, and serialising it is a loop:

```rust
fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for value in embedding {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}
```

Reading it back is the same in reverse. Why SQLite, specifically? Because you already have it, it's one
file, and `rusqlite`'s `bundled` feature compiles SQLite in - no system package, no server, no install
step for students. Your metadata *and* your vectors live in the same place, which means one query can
filter by path and search by meaning.

## Searching

Search is what we already built: embed the query (`query: ` prefix), compare with cosine, take the top
few. Running it on the sample:

```text
Loaded 6 markdown files from .../code/rag_corpus
Split them into 18 chunks
Index written to .../ex26_vector_search/output/rag_index.sqlite

Query: "how do I share data between threads?"
Top 5 chunks:
  1. 0.867  02-SystemThreads/Mutexes.md > Sharing Data with Mutexes
  2. 0.853  02-SystemThreads/Channels.md > Sending Data Between Threads with Channels
  3. 0.851  02-SystemThreads/Channels.md > ... > Multi-Producer, Single Consumer Channels
  ...
```

Notice that each result carries its **source and heading**. That's not decoration - it's the difference
between "the model said something" and "the model said something, and here's the page it read". You will
want that for citations, for debugging, and for the permissions conversation later.

## What We Did, and What We Skipped

We now have a complete retrieval pipeline: read the corpus, chunk it, embed it, store it, search it.

What we deliberately *didn't* do is build an index. `search` in this example loads every vector and
computes cosine in a loop - an `O(n)` scan. At a few hundred chunks that's instant and completely fine,
and it's worth doing once by hand so vector search stops being mysterious. But it doesn't scale to
millions of vectors, and at some point you stop writing that loop. Later in the chapter we swap it for
a real index without changing anything else about the pipeline.
