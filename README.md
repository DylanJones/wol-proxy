# wol-proxy

A collection of small Rust utilities that make it easy to run a power hungry
machine only when it's actually needed. The tools in this repository can wake
machines over the network using Wake-on-LAN and proxy connections until the
machine is ready to serve them.

## Components

### `wol`

`wol` listens on a TCP port and proxies incoming connections to a target
machine. If the target machine is asleep the proxy wakes it before forwarding
the connection. Two wake methods are supported:

- Wake-on-LAN magic packet (default)
- ESP32-S2 companion over UDP (USB HID mouse jiggle)

Example:

```bash
wol-proxy wol --mac aa:bb:cc:dd:ee:ff --target 192.0.2.10:22 --bind 0.0.0.0:2222
```

The above command exposes an SSH service on port `2222`. When a client connects
it wakes the real server at `192.0.2.10` (default: WOL magic packet) and then
proxies the SSH session.

To use the ESP32-S2 companion instead (or alongside WOL):

```bash
# Wake using ESP32-S2 companion listening on UDP 3389
wol-proxy wol \
  --mac aa:bb:cc:dd:ee:ff \
  --target 192.0.2.10:22 \
  --bind 0.0.0.0:2222 \
  --wake-method esp32 \
  --esp32-addr 192.0.2.50:3389

# Or send both WOL and ESP32 UDP in parallel
wol-proxy wol \
  --mac aa:bb:cc:dd:ee:ff \
  --target 192.0.2.10:22 \
  --bind 0.0.0.0:2222 \
  --wake-method both \
  --esp32-addr 192.0.2.50

# Or trigger the Matter companion through Home Assistant
wol-proxy wol \
  --mac aa:bb:cc:dd:ee:ff \
  --target 192.0.2.10:22 \
  --bind 0.0.0.0:2222 \
  --wake-method home-assistant \
  --home-assistant-url http://homeassistant.local:8123 \
  --home-assistant-token "<long-lived-access-token>" \
  --home-assistant-entity-id button.nrf52840_wake

# Combine WOL and Home Assistant in parallel
wol-proxy wol \
  --mac aa:bb:cc:dd:ee:ff \
  --target 192.0.2.10:22 \
  --bind 0.0.0.0:2222 \
  --wake-method wol-and-home-assistant \
  --home-assistant-url http://homeassistant.local:8123 \
  --home-assistant-token "<long-lived-access-token>" \
  --home-assistant-entity-id button.nrf52840_wake
```

Notes
- `--esp32-addr` accepts `ip` or `ip:port` (defaults to port `3389`).
- The ESP32-S2 companion firmware lives in `esp32s2-companion/`. See its
  README for build and configuration instructions.
- The nRF52840 Matter companion firmware lives in
  `nrf52840-zephyr-companion/`. Use `--home-assistant-url`,
  `--home-assistant-token`, and `--home-assistant-entity-id` to point the proxy
  at the Home Assistant instance controlling the Matter device.
- `--home-assistant-service` defaults to `button.press`. Override it if your
  Matter entity exposes a different domain/service pair.

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
