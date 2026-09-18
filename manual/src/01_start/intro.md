# Getting Started

So let's quickly make sure we have working setups before we dive in any further.

> The code is in `code/ex01_hello`

Time for a simple "Hello World", in async with Tokio. Start by creating a project:

```bash
cargo new ex01_hello
cd ex01_hello
cargo add tokio -F full
```

And edit your `main.rs` to be a simple async hello world:

```rust
#[tokio::main]
async fn main() {
  println!("Hello world");
}
```

Make sure it runs. We're not learning anything here - we're just making sure that nobody has toolchain
problems before we dive in.

> You can probably guess the output! It prints "Hello world".
