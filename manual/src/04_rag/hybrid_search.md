# Keywords and Meaning Together

> This is in the repo as `ex29_hybrid_search`.

The scoreboard left us with an awkward result: keyword search and semantic search each win questions
the other loses. BM25 nails "memory leak"; embeddings understand "threads corrupting shared data".
So the obvious move is to stop choosing and run both.

## The Obvious Merge Doesn't Work

The tempting thing is to add the two scores together. Don't - the scores aren't the same kind of
number. Cosine similarity is bounded in `0..1`, while BM25 is unbounded and can easily be `10` or
more. Add them and BM25 quietly decides every result. To fix that you'd normalise, or pick a weight,
and then spend your life tuning that weight. There is a better way.

## Fuse the Ranks, Not the Scores

**Reciprocal Rank Fusion (RRF)** throws the scores away and keeps only the *positions*. Each ranked
list gives a document:

```text
score(document) = sum over lists of  1 / (k + rank)
```

where `rank` is the document's position (1st, 2nd, ...) and `k` is a constant. That's the whole
algorithm, and it's about fifteen lines of Rust:

```rust
fn reciprocal_rank_fusion(rankings: &[Vec<String>], k: usize) -> Vec<String> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    for ranking in rankings {
        for (position, path) in ranking.iter().enumerate() {
            let rank = position + 1;
            *scores.entry(path.clone()).or_insert(0.0) += 1.0 / (k + rank) as f64;
        }
    }
    // sort by score, descending
}
```

The magic is in what `k` does. With `k = 60` (the value from the original paper), `1/61` and `1/62`
are nearly the same, so being 1st versus 2nd barely matters. What *does* matter is appearing in
**both** lists. A file that BM25 loves *and* the embeddings love collects points twice and climbs; a
file that only one search loves keeps its single helping and gets overtaken.

That's the intuition to hold onto: **RRF rewards consensus.** It's how you combine two opinions
without deciding which one you trust more - and without tuning a single weight.

## Does It Actually Help?

Same golden set, same scoreboard, on the full notes:

```text
Question                                         bm25  semantic   hybrid
How do I share data between threads?               #2        #7       #2
How do I send data from one thread to ano...       #1        #1       #1
What are the performance costs of locking...       #2        #1       #1
How do I use Rc to avoid lifetime problems?        #1        #1       #1
Why does the borrow checker complain abou...       #1        #1       #1
How do I find and fix a memory leak?               #1        #5       #1
What is async/await?                               #9        #1       #4
What is the difference between a sender a...       #6        #1       #2
How do I stop threads from corrupting sha...       #2        #5       #2
How do I create a mutable variable in Rust?        #1        #1       #1

Method        hit@1   hit@3   hit@5     MRR
bm25 (kw)      0.50    0.80    0.80    0.68
semantic       0.70    0.70    0.90    0.75
hybrid         0.60    0.90    1.00    0.78
```

Read the two halves of that table separately, because they say different things:

* **At hit@1, hybrid is beaten by semantic** (`0.60` vs `0.70`). Fusion doesn't magically find the
  single best answer more often; sometimes it *promotes the wrong consensus*. Look at async/await:
  BM25 had it at `#9`, semantic at `#1`, and fusion split the difference at `#4`. Merging opinions
  can dilute a confident winner.
* **At hit@3, hit@5 and MRR, hybrid wins outright.** It finds the answer somewhere in the top three
  for `90%` of questions and in the top five for all of them, which is what a retrieval stage feeding
  a model actually needs. And remember the reranker is coming: a fused top-5 that reliably *contains*
  the answer is exactly the input a reranker wants.

So the honest summary is: hybrid trades a little top-1 sharpness for noticeably better recall. For a
RAG pipeline that hands the top chunks to a model, that's usually the right trade.

## The Worked Example

The program spells one question out in full, so you can watch fusion happen. For "How do I stop
threads from corrupting shared data?", searched as the keyword `mutex`:

```text
expected:          Mutexes.md
BM25:              03-Async/SharedState.md, 02-SystemThreads/Mutexes.md, ...
semantic:          02-SystemThreads/README.md, 04-Memory/ReferenceCounting.md, ...
fused:             04-Memory/ReferenceCounting.md, 02-SystemThreads/Mutexes.md, ...
```

Neither search ranked the right answer first. BM25's top pick was `SharedState.md`; semantic's was a
table-of-contents page. But `Mutexes.md` and `ReferenceCounting.md` appeared in *both* lists, so they
collected points from both and rose to the top - which is precisely the behaviour we wanted, on a
question where neither parent search was right on its own.

## The Limits

* **RRF forgets how much.** It uses position only, so a document that won by a landslide is treated
  the same as one that scraped in. Sometimes that matters.
* **`k` is a knob.** `60` is a sane default that nobody argues with; smaller values make the top
  positions matter more.
* **Garbage in, garbage out.** Fusion can only reorder the lists you give it. If neither search found
  the answer, RRF won't invent it - and it can still promote a confident wrong answer by consensus.
* **You pay for both searches.** Two indexes, two query paths. For a small corpus that's nothing; at
  scale it's real infrastructure.

There's one more trick up our sleeve, though. Both searches score the query and the document almost
separately - so neither ever really *reads* the pair together. Next we add a model that does.
