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

#### ESP32-S2 Companion Support

`wol` now supports an ESP32-S2 companion device as an alternative wake method.
This is useful when traditional Wake-on-LAN doesn't work (e.g., on newer computers
or in certain network configurations).

**ESP32-S2 Companion Mode:**
```bash
# Use ESP32-S2 companion with WOL fallback
wol-proxy wol --mac aa:bb:cc:dd:ee:ff --target 192.0.2.10:22 --bind 0.0.0.0:2222 \
  --esp32-ip 192.168.1.100 --esp32-port 9999

# Use ESP32-S2 companion only (no WOL fallback)
wol-proxy wol --mac aa:bb:cc:dd:ee:ff --target 192.0.2.10:22 --bind 0.0.0.0:2222 \
  --esp32-ip 192.168.1.100 --esp32-only
```

### `keepawake`

`keepawake` is a lightweight TCP proxy that holds a system wake lock while a
connection is active and for a short period afterwards. This is useful to keep a
machine from suspending in the middle of a long running connection.

Example:

```bash
wol-proxy keepawake --target 192.0.2.10:80 --bind 0.0.0.0:8080
```

### ESP32-S2 Companion Program

The repository includes a companion program designed to run on an ESP32-S2 
microcontroller. This device connects via USB to the target computer and 
emulates a mouse to wake it when receiving UDP commands from the main wol-proxy.

**Key Features:**
- Connects to Wi-Fi and listens for UDP wake commands
- Emulates USB mouse movement to wake connected computer
- Configurable UDP port (default: 9999)
- Wi-Fi credentials configured via JSON file (not committed to repository)
- Built with Rust using the esp-rs ecosystem

**Setup:**
1. Create `esp32-companion/config.json` based on `config.json.example`
2. Configure your Wi-Fi credentials
3. Build and flash to ESP32-S2:
   ```bash
   cd esp32-companion
   # Configure for ESP32-S2 target (requires esp-rs setup)
   cargo build --release --target xtensa-esp32s2-espidf
   ```

**Configuration:**
```json
{
    "wifi": {
        "ssid": "YOUR_WIFI_SSID", 
        "password": "YOUR_WIFI_PASSWORD"
    },
    "wake": {
        "udp_port": 9999
    }
}

## Building

This project uses [Cargo](https://doc.rust-lang.org/cargo/) for the main wol-proxy
binaries and requires additional setup for the ESP32-S2 companion.

### Main Binaries

To build the main `wol` and `keepawake` binaries:

```bash
cargo build --release
```

The compiled binaries can be found in `target/release/`.

### ESP32-S2 Companion

The ESP32-S2 companion requires the [esp-rs](https://esp-rs.github.io/book/) 
toolchain. Follow the [ESP-RS installation guide](https://esp-rs.github.io/book/installation/index.html) 
to set up the development environment.

```bash
# Install ESP-RS toolchain (one-time setup)
cargo install espup
espup install

# Configure environment
. $HOME/export-esp.sh

# Build for ESP32-S2
cd esp32-companion
cargo build --release --target xtensa-esp32s2-espidf

# Flash to device (requires espflash)
cargo install espflash
espflash flash --monitor target/xtensa-esp32s2-espidf/release/esp32-companion
```

**Note:** The ESP32-S2 companion can also be built and tested on standard platforms 
for development purposes, but USB HID functionality requires actual ESP32-S2 hardware.

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
utilities and ESP32-S2 integration. Run all tests with:

```bash
cargo test
```

**Test Categories:**
- **Unit tests**: Core functionality for each binary
- **Integration tests**: CLI testing for `wol` and `keepawake` 
- **ESP32-S2 tests**: ESP32-S2 companion functionality and integration

To run only ESP32-S2 related tests:

```bash
# ESP32-S2 companion unit tests
cargo test --package esp32-companion

# ESP32-S2 integration tests
cargo test --test wol_esp32_integration
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
