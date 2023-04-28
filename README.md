<div align="center">
  <h1><code>trinity</code></h1>

  <p>
    <strong>Matrix bots in Rust and WebAssembly</strong>
  </p>

  <p>
    <a href="https://github.com/bnjbvr/trinity/actions?query=workflow%3ARust"><img src="https://github.com/bnjbvr/trinity/workflows/Rust/badge.svg" alt="build status" /></a>
    <a href="https://matrix.to/#/#trinity:delire.party"><img src="https://img.shields.io/badge/matrix-join_chat-brightgreen.svg" alt="matrix chat" /></a>
    <img src="https://img.shields.io/badge/rustc-stable+-green.svg" alt="supported rustc stable" />
  </p>
</div>

## TL;DR

Trinity is an experimental bot framework written in Rust and using matrix-rust-sdk, as well as
commands / modules compiled to WebAssembly, with convenient developer features like modules
hot-reload.

## What is this?

This started as a fun weekend project where I've written a new generic Matrix bot framework. It is
written in Rust from scratch using the fantastic
[matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk) crate.

Bot commands can be implemented as WebAssembly components, using
[Wasmtime](https://github.com/bytecodealliance/wasmtime) as the WebAssembly virtual machine, and
[wit-bindgen](https://github.com/bytecodealliance/wit-bindgen) for conveniently implementing
interfaces between the host and wasm modules.

See for instance the [`uuid`](https://github.com/bnjbvr/trinity/blob/main/modules/uuid/src/lib.rs)
and [`horsejs`](https://github.com/bnjbvr/trinity/blob/main/modules/horsejs/src/lib.rs) modules.

Make sure to install [`cargo-component`](https://github.com/bytecodealliance/cargo-component) first
to be able to build wasm components. We're using a pinned revision of this that can automatically
be installed with `./modules/install-cargo-component.sh` at the moment; we hope to lift that
limitation in the future.

Modules can be hot-reloaded, making it trivial to deploy new modules, or replace existing modules
already running on a server. It is also nice during development iterations on modules. Basically
one can do the following to see changes in close to real-time:

- run trinity with `cargo run`
- `cd modules/ && cargo watch -x "component build --release"` in another terminal 

The overall generic design is inspired from my previous bot,
[botzilla](https://github.com/bnjbvr/botzilla), that was written in JavaScript and was very
specialized for Mozilla needs.

## Want / "roadmap"

At this point I expect this to be more of a weekend project, so I won't commit to any of those, but
here's my ideas of things to implement in the future. If you feel like implementing some of these
ideas, please go ahead :)

### Core features

- fetch and cache user names
- make it possible to answer privately / to the full room / as a reply to the original message / as
  a thread reply.
- add ability to set emojis on specific messages (? this was useful for the admin module in botzilla)
- moonshot: JS host so one can test the chat modules on a Web browser, without requiring a matrix
  account
    - marsshot: existing modules built from CI and pushed to a simple Web app on github-pages that
      allows selecting an individual module and trying it.
- seek other `TODO` in code :p

### Modules

- post on twitter. Example: `!tweet Inflammatory take that will receive millions of likes and quote-tweets`
    - same requirements as mastodon, likely
- gitlab auto-link to issues/merge requests: e.g. if someone types `!123`, post a link to
  `https://{GITLAB_REPO_URL}/-/issues/123`.
    - would require the room to be configured with a gitlab repository
    - same for github would be sweet
- ask what's the weather in some city, is it going to rain in the next hour, etc.
- YOUR BILLION DOLLARS IDEA HERE

## Deploy with Docker

If you want, you can use the image published on Docker
([bnjbvr/trinity](https://hub.docker.com/repository/docker/bnjbvr/trinity)) -- it might be lagging
behind by a few commits -- or build the Docker image yourself:

```
docker build -t bnjbvr/trinity .
```

Then start it with the right environment variables (see also `.env.example`):

```
docker run -e HOMESERVER="matrix.example.com" \
    -e BOT_USER_ID="@trinity:example.com" \
    -e BOT_PWD="hunter2" \
    -e ADMIN_USER_ID="@admin:example.com" \
    -v /host/path/to/data/directory:/opt/trinity/data \
    -ti bnjbvr/trinity
```

Data is saved in the `/opt/trinity/data` directory, and it is recommended to make it a volume so as
to be able to decrypt messages over multiple sessions and so on.

If you want, you can specify a custom modules directory using the `MODULES_PATHS` environment
variable and adding another data volume for it. This can be useful for hacking modules only without
having to compile the host runtime. Here's how you can do that:

```
docker run -e HOMESERVER="matrix.example.com" \
    -e BOT_USER_ID="@trinity:example.com" \
    -e BOT_PWD="hunter2" \
    -e ADMIN_USER_ID="@admin:example.com" \
    -e MODULES_PATH="/wasm-modules" \
    -v /host/path/to/data/directory:/opt/trinity/data \
    -v /host/path/to/modules:/wasm-modules \
    -ti bnjbvr/trinity
```

## Is it any good?

Yes.

## Contributing

[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4-ff69b4.svg)](https://www.contributor-covenant.org/version/1/4/code-of-conduct/)

We welcome community contributions to this project.

## Why the name?

This is a *Matrix* bot, coded in Rust and WebAssembly, forming a holy trinity of technologies I
love. And, Trinity is also a bad-ass character from the Matrix movie franchise.

## License

[LGPLv2 license](LICENSE.md).
