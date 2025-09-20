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

1. Install the Zephyr SDK and dependencies (see repository `AGENTS.md` for the
   quickest flow used in automation).
2. Source Zephyr's environment setup, e.g. `source ~/zephyrproject/zephyr/zephyr-env.sh`.
3. From the repository root run:

```sh
west build -b nrf52840dk_nrf52840 nrf52840-zephyr-companion
```

The build outputs the ELF image under `build/zephyr/zephyr.elf`. Use `west flash`
or your preferred Nordic programming tool.

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
