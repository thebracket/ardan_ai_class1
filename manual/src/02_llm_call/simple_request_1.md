# A Simple Request

> The code for this is in `ex02_simple_request`.

Let's start by making the project and adding dependencies:

```bash
cargo new ex02_simple_request
cd ex02_simple_request
cargo add tokio -F full
cargo add reqwest -F json
cargo add dotenvy
cargo add anyhow
cargo add serde_json
```

The dependencies:

* `Tokio` - async runtime. You can use other runtimes.
* `reqwest` - helpful HTTP library.
* `dotenvy` - easy way to read `.env` files.
* `anyhow` - lazy error message handling!
* `serde_json` - for easy JSON work.


