# modules

These are the Trinity modules.

Need to be compiled with the
[`cargo-component`](https://github.com/bytecodealliance/cargo-component) tool, using `./build.sh`.

For hot-reloading, use the `cargo-watch` wrapper script `./watch.sh`, which will recompile as soon
as there's a change to one of the underlying Rust source files. The wasm host will pick up the
changes as soon as they happen, and hot-reload them.

Right now, we're pinning the version of `cargo-component` we're using to a specific revision: use
`./install-cargo-component.sh` from this directory to get the right version.
