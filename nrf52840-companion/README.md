nRF52840 Companion (Thread/802.15.4) — Scaffold

Status
- Compiles for target thumbv7em-none-eabihf using nrf52840-hal.
- Currently a minimal LED blink to validate toolchain. Next we will wire up OpenThread and a USB HID mouse that jiggles on UDP receipt over Thread.

Build

```fish
rustup target add thumbv7em-none-eabihf
cargo build --manifest-path nrf52840-companion/Cargo.toml --target thumbv7em-none-eabihf
```

Flash

```fish
# Requires probe-rs and a supported SWD probe
probe-rs run --chip nRF52840_xxAA --protocol swd --speed 4000 -- nrf52840-companion/target/thumbv7em-none-eabihf/debug/nrf52840-companion
```

UF2 (Adafruit/Seeed UF2 bootloader)

- Bootloader expects application at 0x27000 with SoftDevice S140 v7. Memory layout is configured in `memory.x`.

Quick way to build + convert:

```
# From project folder, produce nrf52840-companion.uf2 in release mode
bash scripts/build-uf2.sh
```

Notes
- The script builds release, converts ELF -> BIN via `arm-none-eabi-objcopy`, then BIN -> UF2 using `u2fconv.py` with family 0xADA52840 and base 0x27000.
- Requires `python3` and `arm-none-eabi-objcopy` in PATH. Install LLVM tools via `rustup component add llvm-tools-preview` if needed.

To flash, double-tap reset to enter UF2 bootloader and drag-drop the `.uf2` onto the UF2 drive that appears.
