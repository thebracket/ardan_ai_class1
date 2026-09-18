# Multi-Turn Chats

When you call an LLM, you're talking to a fresh instance. The LLM system itself is stateless. So if you simply chain a bunch of calls together, there won't be any context - the LLM can't
refer back to previous parts of the chat. When you use Claude, ChatGPT, Gemini etc. your previous chat messages (in both directions, but often not including reasoning) are sent *again* with each request.

If you think back to the request we sent, it looked like this:

```json
"messages" : [
    {
        "role": "user",
        "content": prompt
    }
],

```

When you send a continuing conversation, it looks more like this (and we'll include a system prompt this time!):

```json
"messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hi, my name is Alex."
    },
    {
      "role": "assistant",
      "content": "Hello Alex! How can I help you today?"
    },
    {
      "role": "user",
      "content": "What is my name?"
    }
  ]
```

> This is part of why long conversations can be troublesome. The LLM only has so much *context window* - and replaying large chats each time fills more and more tokens.
