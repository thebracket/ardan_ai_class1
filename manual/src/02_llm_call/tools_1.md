# Tool Use

Chatbots are pretty cool. Chatbots that can wield tools are very useful, and potentially quite dangerous!

The OpenAI standard specifies tools as part of the request in two places. There's a section in the wrapper defining what tools are *available*, reply
messages indicating that the model would like to execute a tool, and messages indicating the results of a tool call.

For example, a POST request with available tools looks like this:

```json
{
  "model": "some-model",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "What's the weather in Columbia?"
    }
  ],

  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get the current weather for a location",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {
              "type": "string",
              "description": "City and state or country"
            }
          },
          "required": ["location"]
        }
      }
    }
  ],

  "tool_choice": "auto"
}
```

When the model wants to make a call, it sends:

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": null,
      "tool_calls": [{
        "id": "call_abc123",
        "type": "function",
        "function": {
          "name": "get_weather",
          "arguments": "{\"location\":\"Columbia, MO\"}"
        }
      }]
    }
  }]
}
```

It's your program's responsibility to actually *run* the tool. You aren't uploading tool programs to OpenRouter, OpenAI, etc. They run on whatever computer is running your harness. When a tool completes, you reply with:

```json
[
  {
    "role": "assistant",
    "content": null,
    "tool_calls": [{
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"location\":\"Columbia, MO\"}"
      }
    }]
  },
  {
    "role": "tool",
    "tool_call_id": "call_abc123",
    "content": "{\"temperature\":72,\"conditions\":\"sunny\"}"
  }
]
```
