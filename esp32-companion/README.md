# ESP32-S2 Companion

This is an ESP32-S2 companion program that provides an alternative wake method for the wol-proxy by emulating USB HID mouse movement.

## Features

- **USB HID Mouse Emulation**: Moves the mouse cursor slightly to wake computers when WOL doesn't work
- **Wi-Fi connectivity**: Connects to your local network to receive wake commands
- **UDP server**: Listens for wake commands from the main wol-proxy
- **Build-time configuration**: Wi-Fi credentials are embedded at compile time for embedded systems
- **Cross-platform testing**: Works on standard platforms for development and testing

## Configuration

### Build-time Configuration (Recommended for ESP32-S2)

For embedded systems like ESP32-S2, configuration is embedded at build time:

1. Copy `config.json.example` to `config.json`
2. Edit `config.json` with your Wi-Fi credentials:
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
   ```
3. Build the project - configuration will be embedded in the binary

**Note**: `config.json` is gitignored to prevent accidentally committing Wi-Fi credentials.

### How it Works

The `build.rs` script reads `config.json` during compilation and generates an `embedded_config.rs` file with compile-time constants. This eliminates the need for filesystem access on embedded systems.

## Building

### For Standard Platforms (Development/Testing)
```bash
cargo build
cargo test
```

### For ESP32-S2 (Future)
```bash
# This would require esp-rs toolchain setup
cargo build --target xtensa-esp32s2-none-elf
```

## USB HID Implementation

The USB HID implementation provides:
- Mouse movement emulation (1 pixel right, then 1 pixel left)
- Minimal wake gesture that doesn't interfere with user activity
- Cross-platform compatibility (mock implementation for testing)

### ESP32-S2 Specific Features
- Uses `usbd-hid` for USB HID device emulation
- Implements proper USB HID mouse reports
- Designed for ESP32-S2's native USB capabilities

## Integration with wol-proxy

The companion works with the main wol-proxy using these new command-line options:

```bash
# ESP32-S2 with WOL fallback
wol --mac aa:bb:cc:dd:ee:ff --target 192.0.2.10:22 --bind 0.0.0.0:2222 \
  --esp32-ip 192.168.1.100 --esp32-port 9999

# ESP32-S2 only (no WOL fallback)
wol --mac aa:bb:cc:dd:ee:ff --target 192.0.2.10:22 --bind 0.0.0.0:2222 \
  --esp32-ip 192.168.1.100 --esp32-only
```

## Development

The codebase supports both embedded and standard platforms through conditional compilation:
- `#[cfg(target_os = "espidf")]` for ESP32-S2 specific code
- `#[cfg(not(target_os = "espidf"))]` for mock implementations and testing

## Security

- Wi-Fi credentials are only stored in `config.json` which is gitignored
- Configuration is embedded at build time, no runtime secrets
- UDP communication is simple and stateless