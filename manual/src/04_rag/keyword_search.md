# Keyword Search

> This is in the repo as `ex25_text_search`.

Before embeddings, before indexes, before anything clever: the oldest search there is. Does the text
contain the word?

## The Simplest Thing That Could Work

Strip the query down to words, lowercase everything, and count how often they appear. That's the whole
algorithm:

```rust
fn keyword_search<'a>(
    documents: &'a [Document],
    query: &str,
    k: usize,
) -> Vec<(&'a Document, usize)> {
    let terms = query.to_lowercase().split_whitespace().collect::<Vec<_>>();

    let mut hits = Vec::new();
    for document in documents {
        let haystack = document.body.to_lowercase();
        let mut count = 0;
        for term in &terms {
            count += haystack.matches(term.as_str()).count();
        }
        if count > 0 {
            hits.push((document, count));
        }
    }

    hits.sort_by(|left, right| right.1.cmp(&left.1));
    hits.truncate(k);
    hits
}
```

No dependencies, no model, no index. Just `String::contains` with arithmetic around it. Searching a
handful of documents for `"memory leak"` gives:

```text
Loaded 6 markdown files from .../code/rag_corpus
Query: "memory leak"

Keyword search (top 5):
    11  04-Memory/MemoryLeaks.md
     1  03-Async/AsyncAwaitOverview.md
```

The right document comes first, by a margin of eleven matches to one. It found it instantly, and it
cost nothing.

## Why This Is Not a Toy

It's tempting to treat this as the naive baseline you graduate away from. Don't. Plain keyword search
is genuinely excellent at a whole class of queries, and it has properties no embedding model has:

* **Exactness.** `Arc<Mutex<T>>`, an error code, a function name, a person's name, a config key - these
  are strings, not concepts. A literal match is *better* than a semantic one here, and an embedding
  model will happily return something "similar" and wrong.
* **It explains itself.** You can show a user exactly which words matched and where. That's how you
  render clickable, trustworthy results - and it's hard to do with a vector.
* **It rewards knowing the vocabulary.** Anyone who grew up picking search terms has a real skill here:
  type the right word and you get the right page. A good keyword search pays that skill off.
* **It's free and instant.** No model download, no per-query cost, no network. For a small corpus, a
  full scan is faster than you can blink.

## Where It Falls Over

All the places where the words aren't the meaning:

* **Synonyms.** The page says `run`; the user types `execute`. Nothing matches.
* **Morphology.** `run`, `running`, `ran` - they're different strings, so they're different terms. (Our
  simple version makes this *worse* by counting substrings, so `run` also matches inside `running` -
  sometimes a feature, often noise.)
* **Typos.** `recieve` finds nothing.
* **Other languages.** A perfectly good Japanese query matches an English page zero times.
* **Common words.** Because we count *every* term, a question like "how do I find and fix a memory
  leak?" is dominated by `how`, `do`, `i`, and the answer drowns. Which is exactly why the next page
  exists.

## The Point

The point isn't that keyword search is weak. It's that **keyword search is the baseline everything else
has to beat** - and, as the scoreboard later shows, it's a hard baseline to beat when the query is a
few good words. Keep it. It's fast, it's explainable, and it's often right.
