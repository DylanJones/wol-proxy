cmake_minimum_required(VERSION 3.20.0)
find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(nrf52840_matter_companion)

target_sources(app PRIVATE 
    src/matter_main.cpp
    src/usb_support.c
)
