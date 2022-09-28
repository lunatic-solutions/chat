### Lunatic.chat

A telnet chat server written in Rust, running on [Lunatic](https://github.com/lunatic-solutions/lunatic).

~I wrote a blog post about the implementation,
[you can read it here](https://lunatic.solutions/blog/lunatic-chat/)~. The implementation has significantly
changed since the blog was written and updated to the [new higher-level process architecture][0] in lunatic.

<div align="center">
    <img src="https://raw.githubusercontent.com/lunatic-solutions/chat/main/assets/ss.png" alt="Terminal look example">
</div>

If you just would like to try it out, join the hosted version with:

```bash
telnet eu.lunatic.chat
```


### Architecture

The server is written in Rust. The Rust code is then compiled to WebAssembly and runs on top of
Lunatic. Each connection runs in a separate (lightweight) process, has its own state and sends
just a diff of esc-sequences back to the terminal to bring it up to date with the current render
buffer.

#### Process architecture:

<div align="center">
    <img src="https://raw.githubusercontent.com/lunatic-solutions/chat/main/assets/diagram.png" alt="Architecture diagram">
</div>

Each rectangle represents a process. The `ClientProcess` holds the current render state that can be changed
by new commands coming from telnet or new messages from channels that the client joined. The `CoordinatorSup`
is a supervisor that will restart the global coordinator if it dies. All processes that depend on the
coordinator are linked to it, if it dies it will disconnect all clients and kill all channels.

### Build & run instructions

If you have [rustup](https://rustup.rs/) installed:

```bash
# Add the wasm32-wasi target
> rustup target add wasm32-wasi
# Build the project
> cargo build --target=wasm32-wasi
```

To run it, you will need to have [lunatic](https://github.com/lunatic-solutions/lunatic) on your PATH.
If this is the case you can just run `cargo run` or find the generated `telnet-chat.wasm` file
in the target folder and run it with `lunatic path/to/telnet-chat.wasm`.

### Licence

MIT

[0]: https://github.com/lunatic-solutions/rust-lib/releases/tag/v0.9.0
