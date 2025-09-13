# wol-proxy

A collection of small Rust utilities that make it easy to run a power hungry
machine only when it's actually needed. The tools in this repository can wake
machines over the network using Wake-on-LAN and proxy connections until the
machine is ready to serve them.

## Components

### `wol`

`wol` listens on a TCP port and proxies incoming connections to a target
machine. If the target machine is asleep the proxy first sends a Wake-on-LAN
packet and waits for the machine to come online before forwarding the
connection.

Example:

```bash
wol-proxy wol --mac aa:bb:cc:dd:ee:ff --target 192.0.2.10:22 --bind 0.0.0.0:2222
```

The above command exposes an SSH service on port `2222`. When a client connects
it wakes the real server at `192.0.2.10` and then proxies the SSH session.

### `keepawake`

`keepawake` is a lightweight TCP proxy that holds a system wake lock while a
connection is active and for a short period afterwards. This is useful to keep a
machine from suspending in the middle of a long running connection.

Example:

```bash
wol-proxy keepawake --target 192.0.2.10:80 --bind 0.0.0.0:8080
```

## Building

This project uses [Cargo](https://doc.rust-lang.org/cargo/). To build the
binaries run:

```bash
cargo build --release
```

The compiled binaries can be found in `target/release/`.

### Cross-compilation

The project supports cross-compilation for multiple platforms. Statically linked
binaries for aarch64 Linux and amd64 Windows are automatically built and
published via CI/CD.

To build for specific targets locally:

```bash
# Install cross-compilation tool
cargo install cross

# Build for aarch64 Linux (statically linked)
cross build --target aarch64-unknown-linux-musl --release

# Build for amd64 Windows
cross build --target x86_64-pc-windows-gnu --release
```

## Testing

The repository contains unit and integration tests covering the proxying
utilities. Run all tests with:

```bash
cargo test
```

**Note**: Tests are required to pass before code can be merged. The CI pipeline
runs both tests and clippy checks on all pull requests.

## CI/CD

The project uses GitHub Actions for continuous integration and deployment:

- **Tests**: All unit and integration tests must pass
- **Linting**: Code must pass `cargo clippy` without warnings
- **Cross-compilation**: Automatically builds statically linked binaries for:
  - `aarch64-unknown-linux-musl` (ARM64 Linux)
  - `x86_64-pc-windows-gnu` (AMD64 Windows)
- **Artifacts**: Built binaries are published as artifacts on the main branch

## License

This project is licensed under the terms of the MIT license. See the
[LICENSE](LICENSE) file for details.
