# Who's Allowed to Read This?

There's one question the rest of this chapter has quietly ignored. Every search we wrote took a query
and returned the best-matching chunks - as if the whole corpus were equally visible to everyone. In a
real system, it isn't. Some documents are for everyone, some are for a team, some are for the person
who wrote them and nobody else. Getting this wrong doesn't produce a bad answer; it produces a *breach*.

## The Model Is Not an Access-Control System

The instinct, once you see the problem, is to put a line in the system prompt: "Only use documents the
user is allowed to see." This does not work, and it is important to understand why. The model cannot
un-read a chunk. Once a confidential passage is in its context, the information is *known*, and a
sufficiently motivated user (or a prompt injection, from the previous page) can often get it back out.
Asking the model to keep a secret it has already been told is not a control; it's a wish.

Access control has to happen **before** the model is involved - at the retrieval step, in ordinary,
deterministic code. If the user can't read a document, that document must never be in the candidate
list.

## Filter at Query Time

The good news is that the machinery is already in place; we've been building it for pages. Every chunk
carries metadata. Add one more field - who may see it - and filter on it *during* the search:

```sql
-- In SQLite, as part of the query itself.
SELECT rowid, distance
FROM vec_chunks
WHERE embedding MATCH ?1
  AND owner IN (SELECT team FROM memberships WHERE user_id = ?2)
ORDER BY distance
LIMIT ?3
```

The same idea lands in LanceDB as `.only_if("...")`, and in any other store as a pre-filter. Two
properties matter:

* **Filter before, not after.** Retrieving the top ten and then discarding what the user can't see
  gives you a short, arbitrary list - and leaks nothing only if you're lucky. Filter *first*, so the
  top ten are ten the user is entitled to.
* **Chunks inherit their parent's permissions.** If a document is restricted, every chunk cut from it
  is restricted. This is far easier to get right at ingest time, when you know where the text came
  from, than to reconstruct later.

## Identity, Not Just a Query

Notice what that required: the retrieval function needs to know *who is asking*. Not a session token
the model can see, but the authenticated user, resolved in your code and passed to the filter. A search
API that takes only a query string cannot do access control, no matter how good it is. Identity is part
of the retrieval contract.

## The Boring, Essential Bits

Beyond the filter itself:

* **Audit.** Log who retrieved what. When a document turns out to have been over-shared, you want to
  know who saw it. This is the same audit trail your
  [Governance in the Real World](../07_governance/in_the_real_world.md) chapter already argues for -
  retrieval is just another place to record.
* **Deletion.** When a document is removed, it has to leave the index and the vector store, not just
  the source folder. Vectors are copies; "deleted" has to mean deleted everywhere.
* **Personal data.** If the corpus contains personal information, the embedding store is now a copy of
  it, with all the retention and privacy obligations that implies. This is a data-protection problem
  the moment you index it, not the moment you search it.
* **Provenance.** Tell the model (and the user) where each chunk came from. It improves answers, and it
  makes over-sharing visible to the people who can do something about it.

## The One-Page Summary

Retrieval is a security boundary. Put identity into the search, filter before you retrieve, make
permissions follow the document into every chunk, and write down what happened. Do all of that in
ordinary code, never in the prompt. The model's job is to answer well; deciding who is allowed to
ask is yours.
