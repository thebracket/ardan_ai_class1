# Retrieval Augmented Generation (RAG)

A model knows two things: what it was trained on, and what you put in the prompt. RAG is about the
second one - finding the right pieces of *your* data and handing them to the model before it answers.

The name is worth unpacking, because it hints at where the work actually is:

* **Retrieval** - find the text that matters.
* **Augmented** - put that text in the prompt.
* **Generation** - let the model answer using it.

Two of those three are old news. Generation is the model's job, and we've spent the whole workshop
getting comfortable with that. Retrieval is a search problem, and we've been building search engines
for decades. A surprising amount of what gets sold as "RAG" is full-text search with extra steps -
and the search is often the part doing the actual work.

So this chapter treats retrieval as the main event, and treats the model as the easy bit at the end.

## Where We're Going

1. **Keyword search, then full-text search.** Finding a word at all, then indexing and ranking the
   hits properly. These are the underpinnings - not a stepping stone to throw away, but the baseline
   everything else has to beat. Often nothing else does.
2. **Embeddings** - what it means to search by meaning instead of by letters, with a picture of the
   vector space so it clicks.
3. **Distance search.** Cosine similarity is the whole engine; once you've written it by hand, vector
   databases stop being magic.
4. **Ingest a real manual**, chunk it sensibly, and keep the vectors in SQLite - a database you
   already have.
5. **Give the bot a tool**, so it can read the docs and answer questions about them.
6. **Measure it.** A small scoreboard for retrieval, so "this is better" is a number and not a vibe.
7. **Combine the two** - keyword and meaning - and check whether the combination earns its keep.
8. **Rerank.** A second model sorts the shortlist before it reaches the chatbot.
9. **The cheap trick.** Small models turn documents into keywords quite well, and keywords are cheap.
10. **When documents fight back.** Retrieved text is untrusted input, and prompt injection lives
    here. This is not a theoretical worry.
11. **Scale.** The `sqlite-vec` extension, then a real LanceDB project.
12. **The unglamorous part.** Most RAG failures are data failures: nothing is tagged, nobody owns it,
    and the permissions are wrong.

## The Corpus

We'll point all of this at a real corpus: my
[Ultimate Rust Foundations](https://github.com/thebracket/ArdanUltimateRust-5Days) class notes. It's
public, it's a few hundred pages of genuine technical prose, and it has the messy structure real
documents have - headings, code blocks, cross-references.

We'll ingest only the prose, not the code. That gives us a natural trap for free: those notes are
forever saying "the code for this is in the `code/...` folder", and the model can't see that folder.
Watch it cheerfully invent the code it can't read.

So the examples run without cloning anything: `code/rag_corpus/` holds one shared sample of the notes,
and every RAG example points at it. Set `RUST_NOTES_DIR` to search a full checkout instead.

The model lives on your machine: everything here uses `fastembed` for embeddings, sparse vectors, and
reranking. The only thing that talks to OpenRouter is the chat model at the end.
