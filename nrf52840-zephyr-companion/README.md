# nRF52840 Zephyr Companion (Matter Wake Mouse)

Zephyr-based firmware for Nordic's nRF52840 that mirrors the functionality of
the Rust companions in this repository. The device now exposes a Matter
On/Off cluster instead of a raw UDP listener. When commissioned into a Matter
fabric (for example, via Home Assistant) any `On` command juggles a USB HID
mouse to wake the connected host. The firmware prints onboarding information
at boot and exposes it over the USB CDC console so the device can be paired
using either a manual code or QR code.

## Features

- Matter endpoint advertising an On/Off cluster that triggers the wake jiggle.
- Automatic generation of manual pairing and QR codes for commissioning.
- Composite USB device: HID mouse for the wake jiggle plus CDC ACM for
  retrieving onboarding information.
- Logging over the default hardware UART console for diagnostics.

## Building

The project uses [PlatformIO](https://platformio.org/) with the Zephyr
framework package pinned to version 4.2.0.

1. Install PlatformIO Core (`pip install platformio` or follow the PlatformIO
   installation guide).
2. From this directory run:

```sh
pio run
```

The resulting images are placed under `.pio/build/nrf52840dk/` (for example
`firmware.elf` and `firmware.hex`). Use `pio run -t upload` or your preferred
Nordic programming utility to flash the board.

> **Note:** Matter support relies on Zephyr's CHIP integration. Ensure your
> environment provides the required Matter libraries when building this sample.

## USB Commands (CDC ACM)

Open the USB serial port at 115200 baud. Each line accepts one command:

- `SHOW` – print the manual code and QR code for Matter commissioning.
- `HELP` – display the list of supported commands.

Unknown commands return an error message. The CDC interface echoes what it
receives so it is easy to script.

## Provisioning

When the device boots it generates and logs a Matter manual pairing code and
QR code. Scan the QR code (or enter the manual code) in Home Assistant's
Matter onboarding flow to add the companion to your fabric. The codes can be
retrieved later by connecting to the USB CDC console and issuing the `SHOW`
command.

## Wake Logic

Receiving an `On` command via the Matter On/Off cluster triggers a mouse
"jiggle": two reports that move the cursor by ±1 on the X axis. After the
wake sequence the cluster resets to `Off` so that subsequent `On` commands can
be issued without additional state changes.
