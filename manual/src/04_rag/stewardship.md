# It's a Data Problem

We've spent this whole chapter on retrieval, and that was the right way to spend the time. But if you
take one thing away from it, take this: **most RAG failures are data failures wearing a retrieval
costume.**

When a bot gives a bad answer, it's tempting to blame the model, or the embedding model, or the
reranker. Usually it's none of them. Usually the right document wasn't there to find, or it was there
three times in different versions, or it was one enormous undifferentiated wall of text, or it was
correct but sitting under a heading that made it look like it belonged somewhere else.

You have already seen the mechanism. The **golden set** in
[Evaluating RAG Results](./evaluating_rag.md) doesn't tell you the model is bad; it tells you which
questions your *data* can't answer. Chunking doesn't fix bad prose; it just decides where to cut it.
No reranker recovers a document the first stage never retrieved.

## Where RAG Actually Works

Look back at the corpora in this chapter. The examples that behave well are the ones with structure:
a title, headings, sections, one idea per page. The failures cluster around the opposite - link lists,
table-of-contents pages, sprawling files with no internal landmarks. That is not a coincidence, and it
is not about model quality.

The uncomfortable summary is: **the places RAG works are the places that already looked after their
data.** If your organisation has a well-ordered, well-named, reasonably current set of documents, RAG
will look like magic. If it has a shared drive full of `final_v3_FINAL(2).docx`, no embedding model on
earth is going to save you - you'll just get wrong answers faster.

## The Checklist

Before you tune a single retrieval parameter, ask what's actually in the index:

* **One canonical source.** Duplicates and near-duplicates don't just waste space; they let stale
  copies outrank current ones, and the model can't tell which is which.
* **Structure.** Headings, short sections, one topic per chunk. This is what makes chunking land on
  meaning instead of an arbitrary character count.
* **Ownership.** Every document should have someone who is responsible for it being right, and for it
  being *retired* when it isn't. An unowned document is a future confidently-wrong answer.
* **Freshness.** How does the index learn that a document changed? Or got deleted? A stale index is
  worse than no index, because it answers with authority.
* **Metadata.** Tags, sections, versions, dates. Metadata is how you filter, how you cite, and how a
  human navigates the results. The cheap keyword extraction from
  [Cheap Tags](./keyword_extraction.md) is one way to manufacture it.
* **Deletion.** When someone asks you to remove a document, it has to leave the index *and* the vector
  store. "Right to be forgotten" is a data problem, not a model problem.

## The Part Nobody Wants to Hear

Data stewardship is slow, unglamorous and un-automatable. It is also the entire difference between a
demo that delights a room and a system that survives contact with real users. You can throw a better
model at a bad corpus every six months forever and it will never work; you can spend that time
tidying the corpus once and the first small model you tried will suddenly look brilliant.

So when someone asks "which embedding model should we use?", the honest first answer is often: *show
me your documents.* The second answer is often a question about who is allowed to read them - which is
where we finish.
