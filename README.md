# snedfile - Rust cross-platform sendfile() abstractions

Natively supported using `sendfile()` are Linux, Fuchsia, FreeBSD and DragonFlyBSD,
and every other `std`-platform using a fallback.

# Usage

This library is designed to make transmitting files as easy as possible.
When you have a file and a TCP stream, all you have to do is

```rust
use snedfile::send_file;

fn transmit(path: impl AsRef<Path>, stream: TcpStream) -> io::Result<()> {
    let file = File::open(path)?;

    send_file(&mut file, &mut stream)
}
```

Trivial errors as well as optimally using the native system capabilities are handled by the implementation.
