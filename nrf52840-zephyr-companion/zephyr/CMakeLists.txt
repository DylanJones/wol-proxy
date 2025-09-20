# PlatformIO compatible CMakeLists.txt for nRF52840 Zephyr Companion
cmake_minimum_required(VERSION 3.20.0)

# Find Zephyr package - PlatformIO handles this automatically
find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})

# Project name
project(nrf52840_zephyr_companion)

# Add source files
target_sources(app PRIVATE 
    src/main.c 
    src/usb_support.c
)