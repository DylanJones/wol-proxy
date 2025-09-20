#!/usr/bin/env bash
# Build and convert to UF2 for nRF52840 Companion (SoftDevice S140 v7, UF2 bootloader)
# Outputs: $NAME.bin and $NAME.uf2 in project root
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing required tool: $1" >&2; exit 127; }; }
need cargo
need arm-none-eabi-objcopy
need python3

# Move to project root regardless of invocation location
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

TARGET=thumbv7em-none-eabihf
NAME=xiao-embassy-hello
BIN="$NAME.bin"
UF2="$NAME.uf2"
UF2_BASE=0x27000
UF2_FAMILY=0xADA52840

echo "[1/3] cargo build --release --target $TARGET"
cargo build --release --target "$TARGET"

echo "[2/3] objcopy elf -> bin"
arm-none-eabi-objcopy -O binary "target/$TARGET/release/$NAME" "$BIN"

echo "[3/3] bin -> uf2 (base=$UF2_BASE family=$UF2_FAMILY)"
python3 ../nrf52840-companion/scripts/u2fconv.py -c -b "$UF2_BASE" -f "$UF2_FAMILY" "$BIN" -o "$UF2"

# slight hack: upload immediately if plugged in
if [ -d /run/media/$USER/XIAO-SENSE ]; then
    python3 ../nrf52840-companion/scripts/u2fconv.py -b "$UF2_BASE" -f "$UF2_FAMILY" "$BIN" -o "$UF2"
fi

bin_size=$(stat -c %s "$BIN" 2>/dev/null || echo "?")
uf2_size=$(stat -c %s "$UF2" 2>/dev/null || echo "?")
echo "Built $BIN ($bin_size bytes) and $UF2 ($uf2_size bytes)"
echo "To flash: enter bootloader (double-tap RESET) and copy $UF2 onto the UF2 drive."
