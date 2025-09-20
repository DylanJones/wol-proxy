# Matter WOL Companion - Deployment Guide

This guide covers deploying the Matter WOL Companion firmware to an nRF52840 development kit and integrating it with Home Assistant.

## Prerequisites

### Hardware
- nRF52840 Development Kit (nrf52840dk_nrf52840)
- USB cable for programming and power
- PC/Mac to wake up (connected via USB)

### Software
- Home Assistant 2022.12+ with Matter integration enabled
- Thread border router (Apple HomePod, Google Nest Hub, Echo devices with Thread, etc.)
- PlatformIO IDE or CLI
- Nordic nRF Connect SDK (for advanced development)

## Firmware Deployment

### 1. Build the Firmware

```bash
# Using PlatformIO CLI
cd nrf52840-zephyr-companion/
pio run

# Output will be in .pio/build/nrf52840dk/
```

### 2. Flash the Device

```bash
# Using PlatformIO
pio run -t upload

# Or using nrfjprog
nrfjprog --program .pio/build/nrf52840dk/firmware.hex --verify --reset
```

### 3. Verify Deployment

1. Connect to USB CDC console at 115200 baud
2. Device should display startup banner
3. Type `INFO` to verify firmware version and status

## Home Assistant Integration

### 1. Prepare Home Assistant

Ensure these prerequisites are met:

- **Matter Integration**: Install from HACS or built-in integrations
- **Thread Support**: Must have a Thread border router on your network
- **Network**: Border router and HA must be on same network

### 2. Commission the Device

1. **Get Device Info**:
   ```
   # Connect to device USB console
   > SETUP
   
   # Note the displayed PIN code: 12345678
   # Note the QR code: MT:Y.K9042C00KA0648G00
   ```

2. **Add to Home Assistant**:
   - Go to **Settings** → **Devices & Services**
   - Click **Add Integration**
   - Search for **Matter**
   - Select **Add device**
   - Choose **Commission with code**
   - Enter setup PIN: `12345678`
   - Wait for commissioning to complete (30-60 seconds)

3. **Verify Integration**:
   - Device should appear as "WOL Companion Button"
   - Check device details show correct vendor/product info
   - Button entity should be available

### 3. Test Wake Functionality

1. **Manual Test**:
   ```
   # USB console
   > WAKE
   # Should see: "Wake triggered manually"
   # PC should wake from sleep
   ```

2. **Home Assistant Test**:
   - Open WOL Companion device in HA
   - Press the button entity
   - PC should wake from sleep
   - Device logs the action

## Integration with Automations

### Basic Automation Example

```yaml
alias: "Wake PC via Matter Button"
description: "Wake gaming PC when button pressed"
trigger:
  - platform: state
    entity_id: button.wol_companion_button
    to: 'on'
action:
  - service: notify.persistent_notification
    data:
      message: "PC wake command sent via Matter device"
      title: "Wake on LAN"
```

### Dashboard Integration

Add the button to your dashboard:

```yaml
type: button
tap_action:
  action: call-service
  service: button.press
  service_data:
    entity_id: button.wol_companion_button
name: Wake PC
icon: mdi:power
```

## Troubleshooting

### Device Not Commissioning

1. **Check Thread Network**:
   - Verify border router is online
   - Check Thread network status in HA
   - Ensure device is in commissioning mode

2. **Reset Commissioning**:
   ```
   # Device console
   > SETUP
   # Re-enter PIN code in Home Assistant
   ```

### Wake Not Working

1. **Test USB HID**:
   ```
   # Device console
   > WAKE
   # Should see mouse movement and wake
   ```

2. **Check PC Settings**:
   - Enable "Allow this device to wake the computer" for USB HID devices
   - Verify USB selective suspend is disabled
   - Test with manual mouse movement

### Connection Issues

1. **Device Console**:
   ```
   > INFO
   # Check HID Status: Ready
   # Check device status
   ```

2. **Thread Network**:
   - Verify Thread credentials are correct
   - Check border router connectivity
   - Monitor device logs for network events

## Advanced Configuration

### Custom Setup Codes

Edit `main/include/CHIPProjectConfig.h`:

```cpp
#define CHIP_DEVICE_CONFIG_USE_TEST_SETUP_PIN_CODE 87654321
#define CHIP_DEVICE_CONFIG_USE_TEST_SETUP_DISCRIMINATOR 1234
```

### Network Settings

Modify `prj.conf` for custom Thread networks:

```ini
CONFIG_OPENTHREAD_CHANNEL=20
CONFIG_OPENTHREAD_PANID=0x1234
CONFIG_OPENTHREAD_NETWORK_NAME="MyNetwork"
```

## Production Deployment

For production use:

1. **Security**: Generate unique device certificates and setup codes
2. **Matter SDK**: Integrate full Matter/CHIP SDK for complete compliance
3. **OTA Updates**: Implement Matter OTA update cluster
4. **Device Attestation**: Use production device attestation certificates

## Support and Development

- **Logs**: Monitor device via USB console and `pio device monitor`
- **Debug**: Enable debug logging in `prj.conf`
- **Updates**: Check project repository for firmware updates
- **Issues**: Report issues with logs and device configuration details