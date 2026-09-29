# Search Is a Distance

> We're still in `ex24_embeddings` in the repo.

We can turn text into a vector. Now: given a query, how do we find the *closest* pieces of text?

The answer is almost disappointing - you score the query against everything you've stored, sort the
scores, and take the best ones. That's it. Everything else in this chapter is about doing that
efficiently and accurately, but the core is three lines of arithmetic.

## Measuring "Closeness"

The standard measure for text is **cosine similarity**:

```text
                     a · b            a₁b₁ + a₂b₂ + ... + aₙbₙ
cosine_similarity = ---------   where  -------------------------- = a · b
                    ‖a‖ × ‖b‖          and ‖a‖ is the vector's length
```

In words: take the **dot product** (multiply the vectors element by element, add it up), then divide
by both vectors' lengths. The result lands between `-1` and `1`, where `1` means "pointing the same
way", `0` means "unrelated", and `-1` means "pointing opposite ways". Real embeddings for real text
almost always sit in the `0`-`1` range, and same-topic text tends to score `0.8` and up.

The important thing is **what it's measuring: direction, not size**. Two documents can be wildly
different lengths - one sentence, one essay - and still be "about" the same thing. Cosine ignores the
length and compares where the vectors point, which is exactly what we want when a long passage and a
short question are semantically the same topic.

### A small correction to the intuition

You'll often hear cosine called "the distance between vectors". It's close, but it's really the
**angle** between them - the cosine of that angle, specifically. That said, the intuition isn't wrong,
and here's the neat part:

```text
distance(a, b) = ‖a − b‖           (ordinary Euclidean distance)

for unit-length vectors:   distance² = 2 − 2 × cosine
```

So **if all your vectors are normalised to length `1`, then "most similar by cosine" and "nearest by
distance" are the same ranking**, just read in opposite directions. The difference only shows up when
lengths vary - and since text length varies a lot, that's precisely why cosine is the default rather
than raw Euclidean distance.

Many embedding models (E5 included) hand you unit-length vectors, so the division is a no-op and the
dot product alone gives the same order. This is why vector databases often advertise "dot product"
support: when vectors are normalised, it's cosine without the cost. If you ever swap metrics and the
results change, the first thing to check is whether your vectors are normalised.

## Top-k: The Shortlist

Scoring gives you a number per document. To make that a *search*, we sort and keep the best few. That
is what **top-k** means:

> **k** is how many results you want. "top-k" means "the k closest matches".

If you ask for the top `5`, you get the five highest-scoring documents, best first. That's the whole
term. It's a leaderboard: score everyone, then read off the top of the table.

Why not pick everything above some score, like `0.85`? Because scores aren't comparable across
queries or models - a `0.85` for one question might be a great match and for another a bad one. A
fixed `k` is predictable and easy to reason about: it's "give me the five best, whatever they score".
In practice you use both - take the top `k`, then optionally drop the ones below a threshold.

## Why It Works: A Real Example

Our little sample set has sixteen words and phrases in four languages. Let's search it for `kitty`,
which **does not appear anywhere in the set**:

```text
Top 5 matches for "kitty":
  1. 0.9520  cat (English)
  2. 0.8972  gato (Spanish)
  3. 0.8969  chat (French)
  4. 0.8911  猫 (Japanese)
  5. 0.8803  dog (English)
```

All four cat words come back before anything else - including the three non-English ones - even though
the query shares no letters with any of them. Try it across a language barrier with `un perro` and the
dog words win. Ask for `a place to live` and the house words take the top three. That's semantic search:
the ranking is by *meaning*, and `kitty` and `猫` are closer to each other than either is to `house`.

### The honest bit

Now look at position five for `kitty`: `dog`, at `0.8803`. It's *wrong*, and it's not far behind the
right answers. This is normal. Top-k will always give you `k` results, even when only one is any good,
and a small model on tiny inputs leaves a lot of noise in the tail.

Two consequences worth carrying forward:

* **Never treat "returned by search" as "relevant".** The model downstream will happily answer using a
  bad chunk if you feed it one.
* **This is why we will measure.** Later we compare keyword search, semantic search, and combinations
  of both - and the only way to know which is winning is to keep score.

## Cost: It's a Linear Scan

For every query we compare against *every* stored vector: `n` documents × `d` dimensions multiplies.
At this scale - sixteen vectors - that's instant. For a few thousand chunks it's still milliseconds. It
only becomes a problem at millions of vectors, and that's precisely when you reach for an index. We'll
get there - first with the `sqlite-vec` extension, then with LanceDB.

But note what those tools are *not* doing: they are not changing the maths above. An index is a clever
way to avoid checking everything, not a different definition of "close". The cosine is still the
cosine.

Next: enough toy words. Let's ingest a real manual.
