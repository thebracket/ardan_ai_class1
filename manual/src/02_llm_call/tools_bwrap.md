# Sandboxes - Bubblewrap

Docker gave us a container. But a container is a fairly heavy thing: a daemon, an image, a build step, and a few hundred megabytes before you get anywhere. Sometimes you want the *containment* without the ceremony - just "run this one command so it can't touch anything it shouldn't".

That is what [Bubblewrap](https://github.com/containers/bubblewrap) (`bwrap`) does. It's a small, unprivileged sandbox built on Linux namespaces, used by Flatpak and a pile of other tools. You hand it a command, plus a description of what that command is allowed to see, and it builds a throwaway filesystem view and runs the command inside it. No daemon, no image, no root required.

This chapter is the same `bash` tool as before, wrapped in `bwrap` instead of Docker. Same lesson, much smaller hammer.

> This is in the repo as `code/ex13_bwrap_sandbox`.

## What `bwrap` gives you

The mental model is a *paint-by-numbers filesystem*. You start with nothing, and then bind in exactly the pieces you want:

* `--ro-bind /usr /usr` - mount the host's `/usr` inside the sandbox, **read-only**. The command can use the programs and libraries, but can't change them.
* `--bind /path/to/work /work` - mount a host directory read-write, at `/work`. This is the one window.
* `--unshare-all` - put the process in its own user, PID, network, IPC and UTS namespaces. It can't see host processes, and it has **no network at all**.
* `--die-with-parent` - if the harness dies, the sandboxed process goes with it. No orphans.
* `--chdir /work` - start in the writable directory.

Anything you don't bind simply isn't there. That's the trick: we don't have to deny access to `/etc/shadow` or your home directory, because we never mounted them. Inside the sandbox they don't exist.

> Bubblewrap is Linux-only. On macOS the equivalent role is played by `sandbox-exec`/Seatbelt, and the tooling around it is different. If you're following along on a Mac, don't worry - there's a Docker route below that gives you a Linux environment to run this in.

## The exercise

We follow the usual workflow. Make the crate, copy the dependencies and the previous source, then change one thing: how `bash` is run.

```bash
cd code
cargo new ex13_bwrap_sandbox
cd ex13_bwrap_sandbox
```

Copy the dependencies and the chat client across from `ex12_docker_safety`, changing the package `name`:

```bash
cp ../ex12_docker_safety/Cargo.toml Cargo.toml   # then rename the package
cp ../ex12_docker_safety/src/chatbot.rs src/chatbot.rs
```

We also want somewhere for the model to work. A plain directory on the host, which we'll expose as `/work`:

```bash
mkdir sandbox
cp ../ex11_bash_tool/secret.txt sandbox/secret.txt
```

That `sandbox/` directory is the entire writable world as far as the model is concerned. Everything else is read-only or absent.

## Building the argument list

The heart of the example is a small function that turns "a directory and a command" into a `bwrap` invocation. It's just a `Vec<String>`, which makes it easy to read and easy to print:

```rust
fn bwrap_args(sandbox_dir: &Path, command: &str) -> Vec<String> {
    let mut args = vec!["--unshare-all".to_string(), "--die-with-parent".to_string()];

    // Read-only system directories, so `bash`, `cat`, `ls` and their libraries
    // are available inside the sandbox.
    for dir in ["/usr", "/bin", "/lib", "/lib64", "/sbin"] {
        if Path::new(dir).exists() {
            args.push("--ro-bind".to_string());
            args.push(dir.to_string());
            args.push(dir.to_string());
        }
    }

    // A minimal /proc and /dev, needed by almost any program.
    args.push("--proc".to_string());
    args.push("/proc".to_string());
    args.push("--dev".to_string());
    args.push("/dev".to_string());

    // The one writable window: the sandbox directory, mounted at /work.
    args.push("--bind".to_string());
    args.push(sandbox_dir.display().to_string());
    args.push("/work".to_string());
    args.push("--chdir".to_string());
    args.push("/work".to_string());

    // Finally, the command to run inside the sandbox.
    args.push("bash".to_string());
    args.push("-c".to_string());
    args.push(command.to_string());

    args
}
```

The `if Path::new(dir).exists()` check is worth noticing. Modern distributions often merge `/bin` and `/lib` into `/usr`, so binding a path that doesn't exist makes `bwrap` refuse to start. Skipping missing directories keeps the example working on both layouts.

Then the `bash` tool itself barely changes - we just run `bwrap` instead of `bash` directly:

```rust
// The command is run through `bwrap`, which creates the sandbox
// and then executes `bash -c <command>` inside it.
let args = bwrap_args(&self.sandbox_dir, &command);
match tokio::process::Command::new("bwrap")
    .args(&args)
    .output()
    .await
{
    Ok(output) => {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // A sandbox denial is a normal tool result, not a crash: `bwrap` (or
        // the command inside it) writes to stderr and we hand that back to the
        // model, which explains it.
        ("output".to_string(), format!("{stdout}{stderr}"))
    }
    Err(err) => ("error".to_string(), err.to_string()),
}
```

Notice how much of the safety comes from *omission*. We never bind the Docker socket, the home directory, or the network, so there is no `-v` flag to get wrong and no capability to remember to drop. The default is "nothing", and we add back only what `bash` needs to run.

And as with Docker, the refusal propagates upwards as data. The model gets `Read-only file system` or `No such file or directory` and reports it to the user; the harness carries on.

## Running it

### On Linux

`bwrap` needs to be installed - it's a normal distro package:

```bash
sudo apt install bubblewrap      # Debian/Ubuntu
sudo dnf install bubblewrap      # Fedora
sudo pacman -S bubblewrap        # Arch
```

Then run the example from `code/`:

```bash
cargo run -p ex13_bwrap_sandbox
```

Unlike the Docker example, there's no image to build and no `--rm` to remember: the sandbox exists only for the lifetime of each command.

### On macOS or Windows

`bwrap` is Linux-only, so if you're not on Linux you can't run the sandbox natively. The pragmatic answer is the same trick as the Docker chapter: put a Linux environment in a container and run the example inside that. The example ships with a `Dockerfile` that does exactly this - it installs `bubblewrap`, runs as an unprivileged user, and copies `sandbox/` in.

There's one wrinkle. Inside a container, `bwrap` is trying to create *nested* namespaces, and Docker's default security profile blocks the namespace and mount operations it needs. The failure is a clear one:

```text
bwrap: No permissions to create new namespace, likely because the kernel does not
allow non-privileged user namespaces.
```

The usual advice is `--privileged`, but that throws away the container's protections wholesale. For a sandboxing lesson that's a bad look. The narrowest set that worked here is three `--security-opt` flags:

```bash
cd code
docker build -t ex13_bwrap_sandbox ex13_bwrap_sandbox

docker run -it --rm \
  --env-file ex13_bwrap_sandbox/.env \
  --security-opt systempaths=unconfined \
  --security-opt seccomp=unconfined \
  --security-opt apparmor=unconfined \
  ex13_bwrap_sandbox
```

What each one is for:

* `systempaths=unconfined` - lets the container remount `/` as `slave`, which `bwrap` needs before it can build its own mount tree. This is the one people miss; without it you get `Failed to make / slave: Permission denied`.
* `seccomp=unconfined` - the default seccomp profile blocks the namespace syscalls `bwrap` uses.
* `apparmor=unconfined` - the host's AppArmor policy can also block user-namespace creation.

Put your key in `code/ex13_bwrap_sandbox/.env` first, exactly as in the other examples.

> This nesting is a genuinely useful thing to demonstrate. It shows that a container and a sandbox are *layers*, not substitutes: the container gives a Mac user a Linux kernel, and `bwrap` then does the fine-grained filesystem work inside it. It also shows why "just use `--privileged`" is the lazy answer - the narrow flags are more instructive, and safer.

You can poke at the nested sandbox by hand, too:

```bash
docker run --rm -it \
  --security-opt systempaths=unconfined \
  --security-opt seccomp=unconfined \
  --security-opt apparmor=unconfined \
  --entrypoint bash ex13_bwrap_sandbox
```

...and from that shell, run the same `bwrap` invocation the tool uses, to watch `/etc/shadow` and the network disappear.

> Heads up: whether nested namespaces are permitted at all is a property of the *host* kernel and Docker configuration. The flags above are the minimum on a standard Linux Docker install; some hardened hosts disable unprivileged user namespaces entirely, in which case no `docker run` flag will help. If you hit that, a small Linux VM is the fallback.

## What to ask it

Same script as the Docker chapter, and the refusals should feel familiar:

```text
Read secret.txt
Read /etc/shadow
Read /home/<you>/.ssh/id_rsa
Try to delete /usr/bin/cat with bash
Fetch https://example.com with curl
```

What you should see:

* `secret.txt` reads fine - it's in `/work`, which is mounted read-write.
* `/etc/shadow` and your SSH key come back as `No such file or directory`. They weren't denied; they were never mounted.
* Deleting `/usr/bin/cat` fails with `Read-only file system`.
* `curl` can't reach anything: `--unshare-all` gave the process its own network namespace, so there's no network to fetch over.

The one that surprises people is the last one. `--unshare-all` doesn't firewall the network, it *removes* it - so there's no route, no DNS, and no interface.

## A note on the Rust crates

You can drive `bwrap` from a crate, but it's worth knowing what's out there before you reach for one:

* The `bwrap` CLI is a stable interface and `std::process::Command` (or Tokio's) is right there. That's why this example does exactly that - no dependency, and the arguments are visible on the projector.
* [`bux-bwrap`](https://crates.io/crates/bux-bwrap) is a small MIT/Apache crate with a typed `BwrapCommand` builder that wraps the flags for you. It also bundles the `bwrap` binary. It's young and lightly used, so check it before betting on it.
* [`birdcage`](https://crates.io/crates/birdcage) is a cross-platform sandbox library (Linux and macOS) with a nice `Exception`-based API - but note it is **GPL-3.0-or-later**, and it implements its own namespaces rather than driving `bwrap`.
* [`bubblewrap-sys`](https://crates.io/crates/bubblewrap-sys) is a `-sys` crate intended to build `bwrap` from a vendored submodule; the crates.io package is a stub and won't build the binary for you.

The general lesson: for something this security-sensitive, understand the underlying command before you let a wrapper hide it. A builder that quietly omits `--unshare-net`, or a bundled binary that behaves differently from the system one, is exactly the kind of thing you want to be able to spot.

## Why this is smaller than Docker

Bubblewrap and Docker are solving the same problem at different weights:

* Docker gives you a portable image, a daemon, resource controls, and a whole ecosystem. You can ship it to a colleague and it just runs.
* Bubblewrap gives you namespaces and bind mounts in a single unprivileged binary. It's perfect for "run this one process with less than it would normally have".

For a tool-running agent, the bubblewrap shape is often the better fit: the model doesn't need an operating system, it needs a *filesystem view* and no network. That's precisely what `bwrap` provides.

## References

* [Bubblewrap on GitHub](https://github.com/containers/bubblewrap) - the source and the security model.
* [`bwrap` manual page](https://man.archlinux.org/man/bwrap.1.en) - every flag, including the ones we didn't use.
* [Bubblewrap on the Arch wiki](https://wiki.archlinux.org/title/Bubblewrap) - a good practical overview.
* [`bwrap` on the Flatpak docs](https://docs.flatpak.org/en/latest/sandbox-permissions.html) - how a large project uses it in anger.
