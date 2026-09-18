# Warning: Unbounded Growth

Before we move on, notice that we made a vector of messages. We only ever add to it, we never delete. If you're in a short, bounded setup - that may be ok. For a regular chatbot,
you will see two problems:

* Eventually, you'll fill the *model's* context window, and it will first become extra stupid - and then stop talking to you with a context size error.
* Likewise, every message back and forth adds to the RAM that your program is using. That's pretty insignificant for regular chat, but if you are in a setup where MANY conversations are growing - or you let them grow unbounded - you could start to have problems.

There's a *lot* of different approaches to context management. We won't implement one yet, but here's some ideas:

* OpenAI expose a "compaction model", whose sole purpose is to read existing conversations and allow them to be replaced with a summary.
* Some systems "middle out" - they keep the beginning and recent context, dropping items from the middle.
* You could just have a hard limit, if you are building a bounded agent.
