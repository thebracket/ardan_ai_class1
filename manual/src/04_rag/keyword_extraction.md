# Cheap Tags: Keywords from Small Models

> This is in the repo as `ex31_keyword_extraction`.

Keyword search is strong, cheap and instant - we've established that. So here's a small trick worth
knowing: what if the documents carried *better* keywords than the ones they happen to contain?

A small, cheap model can read a passage and hand back the handful of words a person would actually
search for. You do this **once, at ingest time**, and then query time stays exactly as fast and free
as ordinary keyword search. This is one of the least glamorous and most useful things you can do with
a language model.

## The Prompt

There is no magic here. It's one instruction, and a formatted reply we can parse:

```rust
let prompt = format!(
    "Extract 1 to {MAX_KEYWORDS} relevant keywords from the following text. \
     Reply with ONLY a JSON array of short, lowercase strings.\n\n\
     text: {}",
    shortened(text, 1500)
);
```

The "reply with only a JSON array" part is what makes it usable - you get data back, not a paragraph
to wrangle. We use a genuinely small model, `meta-llama/llama-3.2-3b-instruct`, which costs a
fraction of a cent per call. Swap it with the `KEYWORD_MODEL` environment variable to try others.

## Running It

With a piece of text as an argument:

```bash
cargo run -p ex31_keyword_extraction -- "A Mutex is like a traffic signal: when you lock a mutex, \
you either gain access to the data, or you wait until the data is available."
```

> mutex, lock, unlock, access, thread

With no argument, it walks the corpus and keywords every chunk - the ingest-time use, and the reason
this is called "cheap tags":

```text
02-SystemThreads/Mutexes.md > Sharing Data with Mutexes
  mutex, thread, atomic, lock, sync
02-SystemThreads/Mutexes.md > Sharing Data with Mutexes > Performance of Mutexes
  mutex, futex, locking, atomic, contention
02-SystemThreads/Channels.md > Sending Data Between Threads with Channels > Multi-Producer...
  mpsc, channel, thread, send, receive
```

Look at the second one: there is no `futex` or `contention` in any keyword list we wrote - the model
pulled them out of the prose. That's the value. Those are real search terms that a reader might type,
and now they're attached to the right page.

## What You Get Out Of It

* **Extra search terms for free.** The chunk picks up synonyms and vocabulary it didn't literally
  contain, which is exactly where plain keyword search is weak.
* **Tags you can show a human.** A list of keywords is a perfect thing to render as clickable topics -
  and unlike an embedding, a user can *see* why a result matched and drive the search themselves.
* **Structured data from prose.** One cheap call turns a paragraph into `["mutex", "thread", "lock"]`.
  That's a building block for indexes, filters, related-page links and nav.

## The Caveats

* **Small models are not reliable.** Every so often ours replies with nothing useful, and you'll see
  `(no keywords)` in the output. That's normal - a 3B model is doing its best. Parse defensively (we
  dig the JSON array out of whatever it says), and treat an empty result as "no extra keywords",
  never as an error worth stopping for.
* **Uncontrolled keywords drift.** Ask for free keywords and you'll get `mutex`, `locks`, `locking`
  and `lock` all as separate terms. That's fine for display, but if you're building an index, give the
  model a fixed list to choose from - a controlled vocabulary - so the tags actually line up.
* **It's cheap, not free.** One call per chunk at ingest. For a big corpus that's real money and real
  time, so cache the results; you don't want to re-keyword everything because the process restarted.
* **It augments, it doesn't replace.** Keep the original text. The keywords are extra search terms on
  top of the document, not a summary of it.

The nice thing about this pattern is how little you need to believe in it. It isn't a claim that
small models are clever; it's just a cheap way to have them do one narrow job well, at a point in the
pipeline where you can inspect the output before anyone depends on it.
