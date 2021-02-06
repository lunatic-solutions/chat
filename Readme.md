### Lunatic.chat

A telnet chat server written in Rust, running on [Lunatic](https://github.com/lunatic-solutions/lunatic).

<div align="center">
    <a href="#">
        <img src="https://raw.githubusercontent.com/lunatic-solutions/chat/main/assets/ss.png" alt="Lunatic logo">
    </a>
    <p>&nbsp;</p>
</div>

If you just would like to try it out, join the hosted version with:

```bash
# US server
telnet lunatic.chat
# EU server
telnet eu.lunatic.chat
```

You should pick the one closer to you as all the rendering is done on the backend and lower latency
will mean better UX.

### Architecture

The server is written in Rust. The Rust code is then compiled to WebAssembly and runs on top of
Lunatic. Each connection runs in a separate (lightweight) process, has it's own state and sends
just a diff of esc-sequences back to the terminal to bring it up to date with the current render
buffer.

### Build & run instructions

TODO

### Licence

MIT
