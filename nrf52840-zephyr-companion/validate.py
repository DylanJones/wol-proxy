#!/usr/bin/env python3
"""
Validation script for nRF52840 Zephyr Companion PlatformIO project.
Checks that the project is properly configured and can build.
"""

import os
import sys
import subprocess
import json

def run_command(cmd, cwd=None):
    """Run a command and return success status and output."""
    try:
        result = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)
        return result.returncode == 0, result.stdout, result.stderr
    except Exception as e:
        return False, "", str(e)

def check_platformio():
    """Check if PlatformIO is installed and working."""
    print("Checking PlatformIO installation...")
    success, stdout, stderr = run_command("pio --version")
    if success:
        print(f"✓ PlatformIO found: {stdout.strip()}")
        return True
    else:
        print(f"✗ PlatformIO not found or not working: {stderr}")
        return False

def check_project_config():
    """Check if the project configuration is valid."""
    print("Checking project configuration...")
    success, stdout, stderr = run_command("pio project config")
    if success:
        print("✓ Project configuration is valid")
        return True
    else:
        print(f"✗ Project configuration error: {stderr}")
        return False

def check_required_files():
    """Check if all required files are present."""
    print("Checking required files...")
    required_files = [
        "platformio.ini",
        "nrf52840_dk.overlay",
        "zephyr/prj.conf",
        "zephyr/CMakeLists.txt",
        "src/main.c",
        "src/usb_support.c",
        "src/usb_support.h"
    ]
    
    all_present = True
    for file_path in required_files:
        if os.path.exists(file_path):
            print(f"✓ {file_path}")
        else:
            print(f"✗ {file_path} missing")
            all_present = False
    
    return all_present

def main():
    """Main validation function."""
    print("nRF52840 Zephyr Companion - PlatformIO Migration Validation")
    print("=" * 60)
    
    all_checks_passed = True
    
    # Check PlatformIO
    if not check_platformio():
        all_checks_passed = False
    
    print()
    
    # Check required files
    if not check_required_files():
        all_checks_passed = False
    
    print()
    
    # Check project configuration
    if not check_project_config():
        all_checks_passed = False
    
    print()
    print("=" * 60)
    
    if all_checks_passed:
        print("✓ All validation checks passed!")
        print("\nTo build the project:")
        print("  pio run")
        print("\nTo build and upload:")
        print("  pio run --target upload")
        return 0
    else:
        print("✗ Some validation checks failed!")
        print("\nPlease fix the issues above before building.")
        return 1

if __name__ == "__main__":
    sys.exit(main())