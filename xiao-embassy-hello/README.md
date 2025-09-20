# xiao-embassy-hello (Seeed XIAO BLE / nRF52840)

A minimal Embassy-based Rust "hello world" for the Seeed Studio XIAO BLE (nRF52840) that blinks the on-board RGB LED and produces a UF2 you can drag-and-drop to the bootloader.

This project targets SoftDevice S140 v7 bootloaders. If your bootloader uses S140 v6, see the note below.

## Prerequisites

- Rust toolchain with the embedded target:
	- rustup target add thumbv7em-none-eabihf
- Arm GNU binutils (for objcopy):
	- arm-none-eabi-objcopy available in PATH
- Python 3 (for UF2 conversion script)
- Seeed/Adafruit UF2 bootloader on the XIAO BLE (double-tap RESET enters bootloader mass-storage mode)

## Board specifics

- Chip: nRF52840
- UF2 Family ID: 0xADA52840
- SoftDevice v7 app base: 0x00027000 (we link and generate UF2 at this address)
- On-board RGB LED pins (active-low):
	- Red: P0.26
	- Blue: P0.06
	- Green: P0.30
	- LOW = LED ON, HIGH = LED OFF

These are encoded in `src/main.rs` and the linker origin is set in `memory.x`.

## Build and generate UF2

Preferred: use the Bash script

```bash
bash scripts/build-uf2.sh
```

This will:
- Build a release binary for `thumbv7em-none-eabihf`
- Convert the resulting ELF to a raw .bin
- Convert .bin to a UF2 at 0x27000 for the nRF52840 family
- Output files next to the project: `xiao-embassy-hello.bin` and `xiao-embassy-hello.uf2`

Cargo alias

- A short build alias is provided:

```bash
cargo b
```

Note: Cargo aliases cannot directly run external shell scripts. If you want `cargo uf2` as a command, add `xiao-embassy-hello/scripts` to your PATH so Cargo discovers `cargo-uf2` (below), then run `cargo uf2`.

Optional: Cargo subcommand helper

```bash
# Temporarily add the scripts dir to PATH for this shell session
export PATH="$(pwd)/scripts:$PATH"

# Now you can run
cargo uf2
```

This simply wraps `scripts/build-uf2.sh`.

## Flashing

1. Double-tap RESET on the XIAO BLE to enter the UF2 bootloader (a USB mass-storage drive will appear).
2. Drag-and-drop `xiao-embassy-hello.uf2` onto the drive.
3. The board will reboot and the RGB LED should blink with staggered colors.

## Manual commands

Run these from the project root (`xiao-embassy-hello/`):

```bash
# Build release for the Cortex-M4F target
cargo build --release --target thumbv7em-none-eabihf

# Convert ELF to raw BIN
arm-none-eabi-objcopy -O binary \
	target/thumbv7em-none-eabihf/release/xiao-embassy-hello \
	xiao-embassy-hello.bin

# Convert BIN to UF2 (S140 v7 base 0x27000, nRF52840 family)
python3 ../nrf52840-companion/u2fconv.py -c \
	-b 0x27000 -f 0xADA52840 \
	xiao-embassy-hello.bin \
	-o xiao-embassy-hello.uf2
```

## SoftDevice v6 vs v7 note

- This project is configured for SoftDevice S140 v7 (app base 0x27000). See `memory.x`:
	- FLASH ORIGIN = 0x27000
- If your bootloader uses SoftDevice S140 v6, change the origin to 0x26000 and also pass `-b 0x26000` to the UF2 converter.

## Files of interest

- `src/main.rs` — Embassy executor with three async blink tasks; correct pins and active-low LED logic
- `memory.x` — FLASH origin set to 0x27000 for S140 v7 bootloader
- `.cargo/config.toml` — cross-compilation target and linker flags; handy cargo alias `b`
- `scripts/build-uf2.sh` — build helper to produce `.bin` and `.uf2`
- `scripts/cargo-uf2` — optional Cargo subcommand wrapper for the script

## Troubleshooting

- If LEDs don’t blink:
	- Confirm you can flash the Zephyr example or any known-good UF2.
	- Ensure SoftDevice version matches the `memory.x` origin and UF2 base address.
	- Verify your toolchain: `arm-none-eabi-objcopy --version` and `python3 --version`.
	- Rebuild cleanly: `cargo clean && cargo build --release --target thumbv7em-none-eabihf`.

- If the UF2 doesn’t appear to program:
	- Confirm the UF2 family is 0xADA52840 and target address 0x00027000 (use a UF2 inspector if needed).

## License

MIT
