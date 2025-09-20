#!/usr/bin/env python3
"""
Demo script showing the Matter WOL Companion USB CDC console interface.

This simulates what a user would see when connecting to the device's 
USB serial port and entering commands.
"""

import time
import sys

def print_banner():
    print("\n" + "="*40)
    print("  Matter WOL Companion v1.0")
    print("="*40)
    print("Type SETUP for Matter commissioning info")
    print("Type HELP for available commands")

def print_setup_info():
    print("\n=== Matter Device Setup ===")
    print("Setup PIN Code: 12345678")
    print("Discriminator: 3840")
    print("Vendor ID: 0xFFF1 (Test Vendor)")
    print("Product ID: 0x8000 (WOL Companion)")
    print()
    print("QR Code: MT:Y.K9042C00KA0648G00")
    print()
    print("Manual Pairing Code: 1234-5678-00")
    print()
    print("=== Home Assistant Setup ===")
    print("1. Open Home Assistant web interface")
    print("2. Go to Settings > Devices & Services")
    print("3. Click 'Add Integration' button")
    print("4. Search for and select 'Matter (BETA)'")
    print("5. Choose 'Add device'")
    print("6. Enter setup PIN: 12345678")
    print("   OR scan the QR code above")
    print("7. Follow the commissioning steps")
    print()
    print("Device will appear as: 'WOL Companion Button'")
    print("Press the button in HA to wake your PC!")
    print()

def print_device_info():
    print("\n=== Device Information ===")
    print("Device: Matter WOL Companion")
    print("Version: 1.0")
    print("HW Version: 1.0")
    print("Status: Ready")
    print("HID Status: Ready")
    print()

def print_help():
    print("\n=== Available Commands ===")
    print("SETUP  - Show Matter commissioning information")
    print("QR     - Show QR code for Matter setup")
    print("INFO   - Show device information")
    print("WAKE   - Trigger wake manually")
    print("HELP   - Show this help")
    print()

def simulate_wake():
    print("Wake triggered manually")
    print("Mouse jiggle sent to USB HID - PC should wake up")

def main():
    print_banner()
    
    while True:
        try:
            command = input("> ").strip().upper()
            
            if command in ["SETUP", "QR"]:
                print_setup_info()
            elif command in ["INFO", "STATUS"]:
                print_device_info()
            elif command in ["WAKE", "TRIGGER"]:
                simulate_wake()
            elif command == "HELP":
                print_help()
            elif command in ["EXIT", "QUIT"]:
                print("Disconnecting from Matter WOL Companion...")
                break
            elif command == "":
                continue
            else:
                print("Unknown command. Type HELP for available commands.")
                
        except KeyboardInterrupt:
            print("\nDisconnecting from Matter WOL Companion...")
            break
        except EOFError:
            break

if __name__ == "__main__":
    print("Matter WOL Companion - USB CDC Console Demo")
    print("This simulates connecting to the device's USB serial port")
    print("=" * 60)
    main()