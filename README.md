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

## Testing

The repository contains unit and integration tests covering the proxying
utilities. Run all tests with:

```bash
cargo test
```

## License

This project is licensed under the terms of the MIT license. See the
[LICENSE](LICENSE) file for details.
