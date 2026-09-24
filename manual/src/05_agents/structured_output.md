# Structured Output Is Not a Contract

Ask a model for JSON and it will usually give you JSON. "Usually" is doing a lot of work in that
sentence.

The failure modes are varied, and some are worse than others:

* **Not JSON at all.** Prose wrapped around the object, a Markdown ```json fence, a trailing "let
  me know if you'd like me to adjust this!", a second object after the first.
* **JSON, but the wrong shape.** A field renamed, a string where you wanted a number, a missing
  key, an array where you expected one object.
* **JSON, and valid, but wrong.** This is the dangerous one. Your schema says `category` is one of
  five values; the model invents a sixth that reads plausibly. Or it returns a perfectly valid
  confidence of `1.4`. Your parser happily accepts it, and your code branches on nonsense.

Many providers offer a structured-output mode - a JSON schema in the request, and a response
constrained to it. This genuinely helps, and you should use it when it's available. But it isn't a
promise, it varies between providers and models, and it doesn't stop valid-but-wrong *values*. A
reasoning model, meanwhile, may emit its thoughts before it emits the object, and you still have to
extract the part you wanted.

So you need a **validation and retry** layer:

1. Parse the output.
2. Validate it against the schema *and* the rules the schema can't express (this enum, this range).
3. If it fails, retry - but **bounded**, and tell the model what was wrong ("`severity` must be one
   of low, medium, high").
4. If it keeps failing, stop. Fall back to something deterministic, or hand it to a person.

The mistake is the unbounded retry: parse failure, try again, parse failure, try again - forever.
It's a loop-guard problem in a different costume, and it costs money every time round. Retries are
a tool, not a strategy.

> We already met the healthy version of this in the structured-output chapter: return data from
> tools so the model reads fields instead of prose, and keep the decisions in your code. Even tool
> calls need their arguments validated - the schema describes the shape, not the sense.

And it's worth asking a bigger question first. A lot of the things we want models to output
"structured" aren't really *generation* at all. They're decisions - and for decisions, there's a
better tool than an LLM.
