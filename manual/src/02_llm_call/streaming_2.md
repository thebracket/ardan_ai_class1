# JSON Reqwest Streams

> The code is in `ex04_streaming`.

Normally, I'd apply some framing and use something like `reqwest_streams` to neatly handle each JSON response. However, looking at the responses - not **all** of them are JSON! Most chunks are:

```
b"data: {\"id\":\"gen-1789578088-PUzMGv5cz124zIBhnJ6j\",\"object\":\"chat.completion.chunk\",\"created\":1789578088,\"model\":\"deepseek/deepseek-v4.1-flash\",\"provider\":\"GMICloud\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\",\"role\":\"assistant\",\"reasoning\":\"We\",\"reasoning_details\":[{\"type\":\"reasoning.text\",\"text\":\"We\",\"format\":\"unknown\",\"index\":0}]},\"finish_reason\":null,\"native_finish_reason\":null}]}\n\n"  
```

And then there's control tags:

```
b"data: [DONE]\n\n"
b""
```

This isn't just a JSON stream, it's a Server Side Event stream. So let's add `reqwest-sse` to our program:

```bash
cargo add reqwest-sse
```

Let's turn our function into something that displays events:

```rust
// At the top:
use reqwest_sse::EventSource;

// And in our function:
   let mut response = client
        .post(URL)
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(message.to_string())
        .send()
        .await?
        .events()
        .await?;

    while let Some(Ok(event)) = response.next().await {
        println!("{event:#?}");
    }
    return Ok(String::new());
```

> Note that this is commented out in the example, as we expand on it right away.

This produces a big stream of event data:

```
Event {
    event_type: "message",
    data: "{\"id\":\"gen-1789584865-JoMt5hKsaCRGDrNjiPuu\",\"object\":\"chat.completion.chunk\",\"created\":1789584865,\"model\":\"deepseek/deepseek-v4.1-flash\",\"provider\":\"BaseTen\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\",\"role\":\"assistant\",\"reasoning\":\"The\",\"reasoning_details\":[{\"type\":\"reasoning.text\",\"text\":\"The\",\"format\":\"unknown\",\"index\":0}]},\"finish_reason\":null,\"native_finish_reason\":null}]}",
    last_event_id: None,
    retry: None,
}
```

So we're receiving events as the `Event` type from the SSE library. The `data` field contains the actual output from the LLM.
