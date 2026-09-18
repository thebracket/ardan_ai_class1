# Understanding the Response

That's quite the response body! Worse, it actually varies slightly by LLM provider. It's the basic OpenAI standard, but it changes a bit.

* `finish_reason` - you want to see `stop`. It might complain about errors, even billing.
* `index` - if there are multiple rounds (we don't support that, yet)
* `logprobs` - you can use this to determine logarithmic probabilities for different responses. Simple requests won't provide this.
* `message` - Aha - some chat from the model!
  * `content` - the full reply.
  * `reasoning` - the reasoning trace.
  * `reasoning_details` - an array of the model's reasoning. Refusals can be documented in here.
* `created` - timestamp.
* `id` - unique trace for the request.
* `model` - the model that was actually used.
* `object` - `chat.completion`; the type of response you are reading!
* `service_tier` and `system_fingerprint` - some providers use this to specify billing details.
* `usage` - counts of the different types of tokens.
* `cost` - how much we spent ($0.00004!); this is OpenRouter specific
* `cost_details` - likewise.
* `is_byok` - is "bring your own key"
* `prompt_token` - how many tokens your prompt became.
* `prompt_token_details` - lets you know about token types.

So a simple HTTP POST request is enough to perform a very simple inference call. It's not very thorough,
and you have to sit and wait for the entire conversation (and possibly adjust timeouts!). And it doesn't
support very much!

So let's build something better.
