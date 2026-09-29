# Let the Bot Read the Docs

> **Draft — planning notes.**
> This is in the repo as `ex27_rag_chat`.

## Covered Here

* Wrapping retrieval in a tool, in the same style as the tool examples in `ex09`/`ex10`.
* The model decides when to search; the tool does the search and hands back the text.
* Building the corpus once at startup: "Building corpus...", then everything is synchronous and
  in-memory, so the tool never has to embed anything mid-conversation.
* Formatting retrieved chunks for the model - source, heading, text.
* Telling the model to answer from the retrieved text and cite it, and knowing it may not.
* Asking questions whose answers exist *only* in the corpus, so we can tell retrieval is doing work.
* The trap: the notes reference `code/...` folders the model cannot see. Watch it invent them.
* Where safety and permissions will have to hook in (foreshadowing the later pages).
