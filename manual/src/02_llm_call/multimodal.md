# Multimodal Input

> This is in the repo as `ex23_multimodal`.

Every message we've sent so far has been text, but there's nothing special about a *string* sitting in
the `content` field. The chat-completions format lets that field be an **array of parts** instead, and
one of those parts can be an image. That's all "multimodal" means at this level: we're still making
the same POST request we made in [A Simple Request](./simple_request_1.md), we've just changed the
shape of one field.

Let's send the model a picture and ask it what it sees.

![](../rust-crab-transparent-bert.png)

## The shape of a multimodal message

A text message looks like this:

```json
{
    "role": "user",
    "content": "Please describe the picture."
}
```

A message carrying text *and* an image looks like this:

```json
{
    "role": "user",
    "content": [
        { "type": "text", "text": "Please describe the picture." },
        {
            "type": "image_url",
            "image_url": { "url": "data:image/png;base64,iVBORw0KGgoAAAA..." }
        }
    ]
}
```

That's it. The text part is the prompt; the image part is the picture. You can have several of each,
and they're processed in order - which is how you do things like "compare the second image to the
first".

## Getting the bytes in there

The API doesn't take a filename. It takes the *bytes* of the image, base64-encoded, inside a
`data:` URL. So there are three small jobs before we build the request:

1. Read the file.
2. Base64-encode it.
3. Work out the MIME type for the `data:` URL (`image/png`, `image/jpeg`, and so on).

We'll lean on the `base64` crate for step two:

```bash
cargo add base64
```

Then:

```rust
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

// A picture travels as base64 text inside the JSON body. Read the file,
// then encode it - the model never sees the path, only the bytes.
let image_bytes = std::fs::read(&image_path)?;
let image_base64 = STANDARD.encode(&image_bytes);

// OpenRouter wants a MIME type in the data URL. We keep it simple and
// infer it from the file extension.
let mime_type = if image_path.ends_with(".jpg") || image_path.ends_with(".jpeg") {
    "image/jpeg"
} else {
    "image/png"
};
```

> Note that nothing here is OpenRouter-specific. The `image_url` part with a base64 `data:` URL is the
> OpenAI chat-completions convention, and most providers accept it. Plenty of providers will also
> accept a plain `https://` URL and fetch the image for you - handy, but it means the model's
> provider is now making a request to whatever is at that address, so we'll stick to bytes we control.

## Building the request

Everything else is the same request we've been making all along. Only the `content` value changes:

```rust
let message = json!(
{
    "model" : model,
    "messages" : [
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": prompt
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{mime_type};base64,{image_base64}")
                    }
                }
            ]
        }
    ],
    "reasoning": {
        "enabled": true
    }
}
);
```

The reply is also exactly what we already know: `choices[0].message.content` is a plain string. Ask
about a picture, get prose back. The model doesn't hand you bounding boxes, labels, or anything
structured unless you ask for that in your prompt (and even then, it's prose - see
[Structured Output Is Not a Contract](../05_agents/structured_output.md) for why you shouldn't trust
it blindly).

Our example takes the picture to send as an optional command-line argument, and falls back to a crab:

```rust
let image_path = std::env::args()
    .nth(1)
    .unwrap_or_else(|| "bert.png".to_string());
```

So `cargo run -- /path/to/your/photo.jpg` works too.

## Sample output

Running it against the crab gives us something like:

> The image shows a cartoon illustration of an orange crab, likely a mascot, that appears to be
> well-read or scholarly. The crab has a spiky shell, large eyes with prominent black pupils, and is
> wearing rectangular black-rimmed glasses. It has a distinctive light pink mustache... The crab is
> perched on top of a stack of two thick books.

Which is a fair description of the picture above. The point is that *we* did nothing image-shaped -
there's no computer vision library, no model download, no preprocessing. We base64'd a file and
changed one field.

## Things worth knowing

* **Images cost tokens.** They show up in the usage block as `image_tokens`, and a decent-sized photo
  is worth a lot more than a sentence of text. Our crab is big enough to notice; resizing a picture
  down before sending it is often the cheapest optimisation you'll make.
* **The model sees pixels, not files.** It can read text *inside* an image (a screenshot, a scanned
  page, a whiteboard), but it can't open the path or fetch anything you didn't send it.
* **Base64 makes the body bigger.** Encoding inflates the bytes by roughly a third before it reaches
  the wire. That's fine for one picture; it's not a reason to skip compression.
* **This composes with everything else.** An image is just another turn in the history, so
  [multi-turn chats](./multi_3.md) work the same way - including the unbounded-growth warning from
  [Warning: Unbounded Growth](./multi_4.md), only now each turn might carry a few hundred kilobytes.
* **It isn't only images.** OpenAI's format also has `input_audio` parts, and some providers accept
  video. The pattern is identical: the `content` array grows a part with a `type` the provider
  understands. If a provider doesn't understand a type, it will usually tell you - or quietly ignore
  it, which is worse.

Once you've got MCP servers returning images (see
[The Round Trip](./mcp_use_9.md)), this is the missing half: turning that base64 blob into an
`image_url` part is all it takes to let the model actually look at it.
