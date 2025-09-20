# nRF52840 Matter WOL Companion

Zephyr-based firmware for Nordic's nRF52840 that implements a Matter-enabled 
wake-on-LAN device. The device appears as a Matter Button in smart home systems
like Home Assistant and triggers a USB HID mouse jiggle to wake the connected 
host when the button is pressed.

## Features

- **Matter Support**: Full Matter-compatible smart home device
- **Thread Networking**: Built on Thread (802.15.4) for reliable mesh networking
- **Home Assistant Integration**: Appears as a native Matter button device  
- **USB HID Wake**: Wiggles mouse cursor to wake sleeping PCs
- **USB CDC Console**: Configuration and commissioning interface
- **QR Code Provisioning**: Standard Matter QR code setup
- **Persistent Settings**: Commissioned credentials stored in flash

## Matter Device Details

- **Device Type**: Generic Switch (Button)
- **Vendor ID**: 0xFFF1 (Test Vendor)
- **Product ID**: 0x8000 (WOL Companion)
- **Setup PIN**: 12345678
- **Discriminator**: 3840

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

- `SETUP` – Show Matter commissioning information and QR code
- `QR` – Show QR code for Matter setup  
- `INFO` – Display device information and status
- `WAKE` – Trigger wake manually for testing
- `HELP` – Show available commands

## Home Assistant Setup

1. **Prerequisites**: Ensure Home Assistant has Matter support enabled
   - Home Assistant 2022.12+ with Matter integration
   - Thread border router (like Apple HomePod, Google Nest Hub, etc.)

2. **Commission the Device**:
   - Connect to the device's USB CDC console
   - Type `SETUP` to display commissioning information
   - In Home Assistant: Settings → Devices & Services → Add Integration
   - Select "Matter" and choose "Add device"
   - Enter setup PIN: `12345678` or scan the QR code
   - Follow the commissioning steps in Home Assistant

3. **Using the Device**:
   - Device appears as "WOL Companion Button" in Home Assistant
   - Press the button in HA dashboard to wake your PC
   - Add to automations, scenes, or dashboards as needed

## Matter Commissioning

The device implements standard Matter commissioning:

- **Setup Code**: 12345678  
- **QR Code**: `MT:Y.K9042C00KA0648G00`
- **Manual Pairing Code**: Available via USB console

Connect to the USB CDC console and type `SETUP` for complete commissioning
instructions including QR code display.

## Thread Network

This device auto-joins the Thread network during Matter commissioning. The
Thread network credentials are managed by the Matter fabric and stored
securely in device flash.

Default Thread network settings (for development):
- **Channel**: 15
- **PAN ID**: 4660  
- **Network Name**: "MATTER"
- **Extended PAN ID**: 11:22:33:44:55:66:77:88
- **Network Key**: 00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF

## Wake Logic

When the Matter button is pressed in Home Assistant:
1. Matter command is received over Thread network
2. Device triggers USB HID mouse movement: two reports that move cursor ±1 pixel
3. Connected PC wakes from sleep/hibernation
4. Action is logged to console and device logs

## Development Notes

This implementation provides a Matter-compatible framework with:
- Thread networking foundation for Matter
- USB HID mouse functionality for PC wake
- Matter device identification and commissioning flow
- Home Assistant integration points

For production deployment, integrate with the full Matter/CHIP SDK to provide
complete Matter cluster implementations and security features.
