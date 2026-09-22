# Talking to AI Models

The most basic step is to call an LLM with a prompt, and receive some output. There are libraries that are designed to do this for you;
we'll talk about those after we've built some ourselves. It's useful to understand what's going on!

For this section, you'll need an OpenRouter key.

> **Note:** If you see an HTTP 402 saying *"This request requires more credits, or fewer max_tokens"*, don't panic - it's
> not a bug in your code, and nothing has run or been billed. OpenRouter performs an up-front credit check: when a request
> doesn't specify `max_tokens`, it assumes the model might run to its maximum output (131072 tokens for
> `deepseek/deepseek-v4.1-flash`) and reserves the worst-case cost of that as collateral. The requests we make here cost a
> fraction of a cent, but that reservation can exceed a small balance - or a key's monthly spending limit.
>
> To get the worst case under your balance, either add credit (or raise the key's limit) at
> [openrouter.ai/settings/credits](https://openrouter.ai/settings/credits), or add a cap to the request JSON:
> `"max_tokens": 4096`. That's plenty for anything we build here, and as we'll see in
> [Warning: Unbounded Growth](./multi_4.md), thinking about token budgets is a good habit to pick up.
