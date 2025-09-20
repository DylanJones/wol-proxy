cmake_minimum_required(VERSION 3.20.0)
find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(nrf52840_zephyr_companion)

set(APP_ROOT ${CMAKE_CURRENT_LIST_DIR}/..)

target_sources(app PRIVATE
    ${APP_ROOT}/src/main.c
    ${APP_ROOT}/src/usb_support.c
)
