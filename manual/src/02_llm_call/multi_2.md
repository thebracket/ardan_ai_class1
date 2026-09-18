# A Quick Refactor

> This is `ex06_refactor` in the repo.

You don't need to re-type everything, but I did a quick refactoring. There's nothing *new*, but it's better laid out. We'll go through the code,
but the primary things to notice are:

* We've moved the chat system into its own file, `chatbot.rs`.
* We've wrapped the chat system in a type, `ChatSession`. This will allow us to start retaining state.
* And because I mentioned it earlier - it makes one `reqwest::Client` and shares it, for a small speed/RAM improvements.

Ok, so let's go ahead and add some conversational state.

