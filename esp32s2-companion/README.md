# ESP32-S2 Companion (USB HID + Wi‑Fi UDP)

This is real embedded Rust firmware for an ESP32-S2. It connects to Wi‑Fi, listens for UDP packets on a configurable port (default 3389), and when a packet is received it emulates a USB mouse “jiggle” over USB to wake the connected host.

Highlights
- ESP-HAL + Embassy + esp-wifi (no mocks).
- USB composite device: HID mouse + CDC serial (for runtime config).
- Configurable UDP port (default 3389; not the WOL port).
- Wi‑Fi credentials stored in NVS. No secrets in repo.

Runtime configuration (preferred)
- Connect the ESP32-S2 over USB. It enumerates as both a mouse and a serial port.
- Open the serial port at 115200 and send either:
  - `WIFI <ssid> <pass>`
  - `PORT <number>`
  - `SHOW` to print current SSID and port
- Alternatively send JSON: `{"ssid":"myssid","pass":"mypass"}` or `{ "port": 4444 }`.
- Settings are saved in NVS and persist across reboots.

Fallback config (optional)
- If runtime serial config is unavailable, you may add a local `secrets.toml` (excluded by .gitignore) and implement a tiny loader, or pre-provision NVS via your flasher tool. This repo does not include credentials.

Build notes
- Requires the Xtensa Rust toolchain and `cargo` setup for `esp32s2` targets.
- Example target triple: `xtensa-esp32s2-none-elf`.
- A typical flow (tooling varies):
  - Install rust toolchain for ESP32-S2 per esp-rs book.
  - `cd esp32s2-companion`
  - `cargo build --release` (with correct target)
  - Flash with your preferred tool (espflash, etc.).

Network notes
- Uses DHCPv4 by default.
- Listens on UDP port 3389 unless changed.
- Packet content: any non-empty UDP payload triggers a wake jiggle. The sender may use the token `WAKE` for clarity, but it is not required.

USB notes
- The device VID/PID is a placeholder (0x1209:0x0001). Adjust for your environment.
- Ensure your host OS allows USB HID to wake the machine.

Directory safety
- `secrets.toml` and similar local files remain untracked by Git per `.gitignore`.

Testing
- Unit tests cover only off-device logic (config parsing and UDP signal decision). The embedded peripherals are not mocked.

