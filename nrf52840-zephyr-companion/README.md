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

This project now uses [PlatformIO](https://platformio.org/) with the Zephyr framework instead of west.

### Prerequisites

1. Install PlatformIO:
   ```bash
   pip install platformio
   ```

2. Install the Nordic nRF52 platform and Zephyr framework (PlatformIO will handle this automatically):

### Building

From the `nrf52840-zephyr-companion` directory:

```bash
# Build the project
pio run

# Build and upload to the device
pio run --target upload

# Clean build artifacts
pio run --target clean
```

The build outputs will be in the `.pio/build/nrf52840_dk/` directory.

### Alternative Build Commands

```bash
# Build for production
pio run --environment nrf52840_dk

# Monitor serial output
pio device monitor --port /dev/ttyACM0 --baud 115200
```

## Configuration

The project uses PlatformIO's Zephyr framework integration. Configuration is handled through:

- `platformio.ini` - Main project configuration
- `zephyr/prj.conf` - Zephyr-specific configuration options
- `nrf52840_dk.overlay` - Device tree overlay for USB configuration

### Customizing the Build

To modify Zephyr configuration options, edit `zephyr/prj.conf`. Key configuration options include:

- `CONFIG_OPENTHREAD_*` - Thread network parameters
- `CONFIG_WOL_LISTEN_PORT` - Default UDP listen port (3389)
- `CONFIG_WOL_USB_MAX_LINE` - Maximum command line length for CDC interface

## USB Commands (CDC ACM)

Open the USB serial port at 115200 baud. Each line accepts one command:

- `PORT <number>` – update the UDP port and persist the value.
- `SHOW` – print the active port value.

Unknown commands return an error message. The CDC interface echoes what it
receives so it is easy to script.

## Thread Network

This sample auto-attaches to a Thread network defined by the constants in
`zephyr/prj.conf`. Update `CONFIG_OPENTHREAD_*` values to match your deployment. If you
prefer commissioning, enable the joiner in `zephyr/prj.conf` and extend the CDC shell
or connect via the OpenThread CLI to supply credentials.

## Wake Logic

Any non-empty UDP payload on the configured port triggers a mouse "jiggle": two
reports that move the cursor by ±1 on the X axis. This matches the behavior of
the Rust implementations.
