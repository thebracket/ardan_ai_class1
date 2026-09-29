# An Index You Don't Have to Write: sqlite-vec

> This is in the repo as `ex32_sqlite_vec`.

Back in [Ingesting a Manual](./ingesting_a_manual.md) we did the honest, dumb thing: store every vector
as a `BLOB`, load the lot into memory, and compute cosine in a loop. At four hundred chunks that's
instant, and it's a great way to see that a vector search is *just arithmetic*.

It stops being great at a million chunks. So at some point you stop writing the loop and reach for an
index - and the nice thing is you don't have to leave SQLite to do it.

## A Real Index Inside SQLite

[`sqlite-vec`](https://github.com/asg017/sqlite-vec) is a loadable extension that adds a `vec0`
virtual table type to SQLite. You still have one database file with your metadata *and* your vectors,
but now there's an actual vector index living beside the rows.

Registration is a single `unsafe` line - after it, `vec0` just exists on every connection:

```rust
unsafe {
    use rusqlite::ffi::sqlite3_auto_extension;
    use sqlite_vec::sqlite3_vec_init;
    sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
}
```

Then the schema is barely different from what we had before. `float[384]` has to match the embedding
model:

```sql
CREATE VIRTUAL TABLE vec_chunks USING vec0(embedding float[384]);
```

Insert the vectors as bytes, exactly as before, and search:

```sql
SELECT rowid, distance
FROM vec_chunks
WHERE embedding MATCH ?1
ORDER BY distance
LIMIT ?2
```

That's the whole index. You hand it a vector, it hands back the nearest rowids.

> **A gotcha worth knowing before you hit it:** vec0 needs to see the `LIMIT` *on its own table*, so a
> KNN query inside a join fails with "a LIMIT or 'k = ?' constraint is required". Do the vector search
> in a subquery, then join the metadata back on. The example does exactly that.

## Running It

```bash
cd code/ex32_sqlite_vec
cargo run
```

```text
Corpus: 18 chunks from .../code/rag_corpus
Index written to .../ex32_sqlite_vec/output/rag_vec.sqlite
sqlite-vec version: v0.1.9

Query: "how do I share data between threads?"
Top 5:
  1. distance 0.5162  02-SystemThreads/Mutexes.md > Sharing Data with Mutexes
  2. distance 0.5420  02-SystemThreads/Channels.md > Sending Data Between Threads with Channels
  ...
```

The search that took a `for` loop over every vector in `ex26_vector_search` is now one SQL statement,
and the same rankings come back.

## The Distance Is Not Cosine

Notice those numbers - `0.5162`, not `0.93`. `vec0`'s default metric for float vectors is **L2
(Euclidean) distance**, not cosine. It doesn't matter for ranking here, because the embedding model
hands us unit-length vectors, and for unit vectors "smallest L2 distance" and "largest cosine" are the
same ordering (recall `distance² = 2 − 2 × cosine`). But if you compare the two examples side by side,
don't panic at the different numbers - read the *order*, not the value.

## What You Gained, What You Didn't

**Gained:** an index you didn't have to write, maintained inside the database you already trust. Your
metadata and vectors sit together, so filtering by a `WHERE` clause (a path, a tag, a permission) is
ordinary SQL. Still local, still one file, still no server.

**Not gained:** scale beyond a single machine. `sqlite-vec` is a superb choice for a personal index, a
desktop app, a workshop, or a corpus in the tens-to-hundreds of thousands of chunks. When you need
billions of vectors, multi-writer access, versioning, object storage, or multimodal data, it's time for
a purpose-built vector database - which is the last page of this chapter.
