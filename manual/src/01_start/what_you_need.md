# What You Need

Before we start, let's get your machine ready. Nothing here is exotic, but a few of the later
chapters reach outside the Rust toolchain, so it's worth sorting this out now rather than
halfway through a live demo.

## Rust

Install Rust with [rustup](https://rustup.rs/) if you haven't already:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Windows users: grab the installer from the same page. Then check it works:

```bash
cargo --version
rustc --version
```

The examples use the 2024 edition, so you want a reasonably recent toolchain. `rustup update`
keeps you current.

## An Editor

Whatever you're comfortable in. If you don't have a favourite, VS Code with the
[rust-analyzer](https://rust-analyzer.github.io/) extension is a popular pairing, and it will
happily explain the compiler errors we're about to generate.

I'll be using Zed, RustRover and/or Helix - because I like them.

> **Aside**: I once made the mistake of explaining how to use an editor in a book. I received more comments "but I use X" than I did on any other part of the book! I'm not making that mistake again - use whatever editor makes you productive.

## An OpenRouter Key

Our chat examples talk to models through [OpenRouter](https://openrouter.ai). Create an
account, add a small amount of credit, and make yourself an API key. (If you skip the credit,
you'll meet an HTTP 402 in chapter two and nowhere to go.)

Put the key in one file, at the top of the example workspace:

```bash
echo 'OPENROUTER_KEY=sk-or-...' > code/.env
```

Every example finds it: `dotenvy` searches the current directory and its parents, so it doesn't
matter which example you run, or where you run it from. (If an example ever needs different
settings, drop a `.env` next to it - the nearest file wins.)

Two habits worth forming straight away:

* Keep the key out of git, screenshots, and the manual. The repo's `.gitignore` already covers
  `.env` files, but it can't protect you from copying one into a slide deck.
* Keep it in `.env` and nowhere else: not in a script, not in a Dockerfile, not in the book.

## Node and npx

The MCP chapters launch a real MCP server over stdio, and the easiest server to play with is a
Node package. Install [Node.js](https://nodejs.org/) - `npx` ships with it - and check:

```bash
node --version
npx --version
```

A current LTS release is fine. You don't need to know any JavaScript: we'll be talking to the
server over stdin and stdout, not writing it.

## Later On

A couple of chapters use extra tools of their own. You won't need them until we reach them:

* **Containers - Docker** needs Docker installed and running.
* **Sandboxes - Bubblewrap** needs Linux with `bwrap` available - we use a Docker fallback on
  other platforms.

That's the lot. On to the hello world.
