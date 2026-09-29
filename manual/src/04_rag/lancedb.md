# When SQLite Isn't Enough: LanceDB

> The project is in the repo as `ex33_lancedb`. We won't build it line by line in class - read it, run
> it, and know it's there when you outgrow SQLite.

`sqlite-vec` takes you a long way: a real vector index, inside a database you already have, with no
server to run. For a lot of systems that is genuinely the end of the story, and you should not feel
any pressure to move past it.

But it is still a single SQLite file on a single machine. It doesn't give you an ANN index tuned for
millions of vectors, multi-writer concurrency, built-in versioning, object-storage backends, or vector
search over images and audio. When you need those, you reach for a dedicated vector database.

## What LanceDB Is

[LanceDB](https://lancedb.com) is an **embedded** vector database - meaning it runs in your process,
like SQLite, rather than as a server you deploy. Underneath, it stores data in the
[Lance](https://lancedb.github.io/lance/) columnar format, which is designed for vectors and
multimodal data rather than invented before they existed.

That combination is the appeal: SQLite's "it's just a library and a directory" ergonomics, with a
storage engine built for what you're actually doing - plus automatic versioning of your data for free.

## The Project

`ex33_lancedb` is the same pipeline you now know - load the corpus, chunk it, embed each chunk - with
only the storage and search layer swapped out. Instead of a `BLOB` column, the table is described with
an Arrow schema, where a vector is a fixed-size list of floats:

```rust
let schema = Schema::new(vec![
    Field::new("path", DataType::Utf8, false),
    Field::new("heading", DataType::Utf8, false),
    Field::new("body", DataType::Utf8, false),
    Field::new(
        "vector",
        DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384),
        false,
    ),
]);
```

You write a `RecordBatch`, ask LanceDB for a table, and search:

```rust
let db = lancedb::connect(&database).execute().await?;
let table = db.create_table("chunks", batch).execute().await?;

let stream = table.vector_search(query_embedding)?.limit(5).execute().await?;
```

The results come back as Arrow record batches - which is why the example has to reach for
`StringArray` and `Float32Array` to read the columns out. That's the price of admission for a
columnar engine, and it's the same shape you'd use to read a Parquet file.

Running it gives the results you'd expect:

```text
Corpus: 18 chunks from .../code/rag_corpus
Table written to .../ex33_lancedb/output/lancedb

Query: "how do I share data between threads?"
Top 5:
  1. distance 0.2665  02-SystemThreads/Mutexes.md > Sharing Data with Mutexes
  2. distance 0.2938  02-SystemThreads/Channels.md > Sending Data Between Threads with Channels
  ...
```

## Where This Goes From Here

The thirty-line example is deliberately the smallest useful thing. Everything the rest of the chapter
argued for has a home in this API:

* **Real indexes.** `table.create_index(...)` builds an ANN index - the thing that makes "millions of
  vectors, milliseconds" true, rather than the brute-force scan we started with.
* **Metadata filtering.** `.only_if("section = 'async'")` filters *during* the search, which is where
  the permissions and tagging conversations from the next pages actually get enforced.
* **Hybrid search.** LanceDB speaks full-text search too, so the keyword-plus-vector fusion you built
  by hand has a native version.
* **Scale and sharing.** Point it at object storage, run it in the cloud, keep versions of the data -
  all without changing the pipeline above.

## The Honest Advice

Don't adopt a vector database because it's fashionable. The pipeline is identical whether the vectors
sit in SQLite or Lance - chunk, embed, search, fuse, rerank - and the earlier stages are where the
quality comes from. Reach for LanceDB when you have a *reason*: the corpus is big enough that brute
force is slow, the data isn't just text, or you need the operational features. Until then,
`sqlite-vec` is not a toy; it's frequently the correct answer.
