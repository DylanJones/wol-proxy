# nRF52840 Zephyr Companion (Thread UDP Wake Mouse)

Zephyr-based firmware for Nordic's nRF52840 that mirrors the functionality of
the Rust companions in this repository. The device joins a Thread network,
listens for UDP datagrams on a configurable port (default 3389), and when a
packet arrives it wiggles a USB HID mouse to wake the connected host. A USB CDC
console provides runtime configuration (currently limited to changing or
querying the UDP port), and the selection is persisted via Zephyr's settings
subsystem (NVS backend).

## Features

- Full Thread (802.15.4) end-device built on Zephyr's OpenThread integration.
- IPv6 UDP listener with simple payload inspection.
- Composite USB device: HID mouse for the wake jiggle plus CDC ACM for
  configuration shell.
- Persistence of the UDP port selection to internal flash using Zephyr
  settings.
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

## USB Commands (CDC ACM)

Open the USB serial port at 115200 baud. Each line accepts one command:

- `PORT <number>` – update the UDP port and persist the value.
- `SHOW` – print the active port value.

Unknown commands return an error message. The CDC interface echoes what it
receives so it is easy to script.

## Thread Network

This sample auto-attaches to a Thread network defined by the constants in
`prj.conf`. Update `CONFIG_OPENTHREAD_*` values to match your deployment. If you
prefer commissioning, enable the joiner in `prj.conf` and extend the CDC shell
or connect via the OpenThread CLI to supply credentials.

## Wake Logic

Any non-empty UDP payload on the configured port triggers a mouse "jiggle": two
reports that move the cursor by ±1 on the X axis. This matches the behavior of
the Rust implementations.
