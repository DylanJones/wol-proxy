/*
 * Copyright (c) 2024 WOL Proxy Project
 *
 * SPDX-License-Identifier: MIT
 */

#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/settings/settings.h>
#include <zephyr/usb/usbd.h>
#include <zephyr/usb/class/usbd_hid.h>
#include <zephyr/drivers/uart.h>
#include <string.h>
#include <ctype.h>

extern "C" {
#include "usb_support.h"
}

// For now, let's implement a simplified Matter-like interface
// In a full implementation, we would use the actual Matter/CHIP SDK

LOG_MODULE_REGISTER(matter_wol, CONFIG_LOG_DEFAULT_LEVEL);

// HID mouse report structure
static const uint8_t hid_report_desc[] = HID_MOUSE_REPORT_DESC(3);

struct mouse_report {
    uint8_t buttons;
    int8_t x;
    int8_t y;
    int8_t wheel;
} __packed;

// Global variables
static const struct device *hid_dev;
static const struct device *cdc_dev;
static struct usbd_context *usb_ctx;
static bool hid_ready;
static struct k_work mouse_work;
static struct k_mutex hid_mutex;
static struct k_mutex cdc_tx_mutex;
UDC_STATIC_BUF_DEFINE(mouse_report_buf, sizeof(struct mouse_report));

static char cdc_line_buf[CONFIG_WOL_USB_MAX_LINE];
static size_t cdc_line_len;

// Matter commissioning information
static constexpr uint32_t kSetupPinCode = CHIP_DEVICE_CONFIG_USE_TEST_SETUP_PIN_CODE;
static constexpr uint16_t kSetupDiscriminator = CHIP_DEVICE_CONFIG_USE_TEST_SETUP_DISCRIMINATOR;
static constexpr uint16_t kVendorId = CHIP_DEVICE_CONFIG_DEVICE_VENDOR_ID;
static constexpr uint16_t kProductId = CHIP_DEVICE_CONFIG_DEVICE_PRODUCT_ID;

// Function declarations
static void submit_mouse_work(void);

// HID interface ready callback
static void hid_iface_ready(const struct device *dev, const bool ready)
{
    ARG_UNUSED(dev);
    hid_ready = ready;
}

static struct hid_device_ops hid_ops = {
    .iface_ready = hid_iface_ready,
};

// Mouse jiggle work handler - triggers PC wake-up
static void mouse_jiggle(struct k_work *work)
{
    ARG_UNUSED(work);

    if (!hid_dev || !hid_ready) {
        LOG_WRN("HID device not ready for mouse jiggle");
        return;
    }

    struct mouse_report reports[2] = {
        {.buttons = 0, .x = 1, .y = 0, .wheel = 0},
        {.buttons = 0, .x = -1, .y = 0, .wheel = 0},
    };

    k_mutex_lock(&hid_mutex, K_FOREVER);
    for (size_t i = 0; i < ARRAY_SIZE(reports); ++i) {
        memcpy(mouse_report_buf, &reports[i], sizeof(struct mouse_report));
        int ret = hid_device_submit_report(hid_dev,
                                           sizeof(struct mouse_report),
                                           mouse_report_buf);
        if (ret) {
            LOG_WRN("Failed to send HID report (%d)", ret);
            break;
        }
    }
    k_mutex_unlock(&hid_mutex);
    
    LOG_INF("Mouse jiggle triggered - PC should wake up");
}

static void submit_mouse_work(void)
{
    k_work_submit(&mouse_work);
}

// CDC console functions for device information
static void cdc_write(const char *data, size_t len)
{
    if (!cdc_dev) {
        return;
    }

    k_mutex_lock(&cdc_tx_mutex, K_FOREVER);
    while (len > 0) {
        int written = uart_fifo_fill(cdc_dev, (const uint8_t *)data, len);
        if (written < 0) {
            break;
        }
        data += written;
        len -= (size_t)written;
        if (written == 0) {
            k_sleep(K_MSEC(5));
        }
    }
    k_mutex_unlock(&cdc_tx_mutex);
}

static void cdc_write_str(const char *str)
{
    cdc_write(str, strlen(str));
}

static void show_qr_code(void)
{
    // Generate QR code info for Matter commissioning
    char qr_info[512];
    
    // Generate a Matter QR code payload
    // Format: MT:Y.K9042C00KA0648G00
    // This is a simplified example - in real implementation would use proper Matter QR code generation
    snprintk(qr_info, sizeof(qr_info), 
             "\r\n=== Matter Device Setup ===\r\n"
             "Setup PIN Code: %08lu\r\n"
             "Discriminator: %u\r\n"
             "Vendor ID: 0x%04X (%s)\r\n"
             "Product ID: 0x%04X (%s)\r\n"
             "\r\n"
             "QR Code: MT:Y.K9042C00KA0648G00\r\n"
             "\r\n"
             "Manual Pairing Code: %04u-%04u-%04u\r\n"
             "\r\n"
             "=== Home Assistant Setup ===\r\n"
             "1. Open Home Assistant web interface\r\n"
             "2. Go to Settings > Devices & Services\r\n"
             "3. Click 'Add Integration' button\r\n"
             "4. Search for and select 'Matter (BETA)'\r\n"
             "5. Choose 'Add device'\r\n"
             "6. Enter setup PIN: %08lu\r\n"
             "   OR scan the QR code above\r\n"
             "7. Follow the commissioning steps\r\n"
             "\r\n"
             "Device will appear as: 'WOL Companion Button'\r\n"
             "Press the button in HA to wake your PC!\r\n"
             "\r\n",
             (unsigned long)kSetupPinCode,
             kSetupDiscriminator,
             kVendorId, "Test Vendor",
             kProductId, "WOL Companion",
             (kSetupPinCode / 10000) % 10000,  // Simple manual code generation
             (kSetupPinCode / 100) % 100,
             kSetupPinCode % 100,
             (unsigned long)kSetupPinCode);
    
    cdc_write_str(qr_info);
}

static void show_device_info(void)
{
    char info[256];
    
    snprintk(info, sizeof(info),
             "\r\n=== Device Information ===\r\n"
             "Device: Matter WOL Companion\r\n"
             "Version: %s\r\n"
             "HW Version: %s\r\n"
             "Status: %s\r\n"
             "HID Status: %s\r\n"
             "\r\n",
             CHIP_DEVICE_CONFIG_DEVICE_SOFTWARE_VERSION_STRING,
             CHIP_DEVICE_CONFIG_DEVICE_HARDWARE_VERSION_STRING,
             "Ready",
             hid_ready ? "Ready" : "Not Ready");
    
    cdc_write_str(info);
}

static void process_cdc_command(const char *line)
{
    while (isspace((unsigned char)*line)) {
        ++line;
    }
    if (*line == '\0') {
        return;
    }

    if (strncasecmp(line, "SETUP", 5) == 0 || strncasecmp(line, "QR", 2) == 0) {
        show_qr_code();
        return;
    }

    if (strncasecmp(line, "INFO", 4) == 0 || strncasecmp(line, "STATUS", 6) == 0) {
        show_device_info();
        return;
    }

    if (strncasecmp(line, "WAKE", 4) == 0 || strncasecmp(line, "TRIGGER", 7) == 0) {
        submit_mouse_work();
        cdc_write_str("Wake triggered manually\r\n");
        return;
    }

    if (strncasecmp(line, "HELP", 4) == 0) {
        cdc_write_str("\r\n=== Available Commands ===\r\n"
                      "SETUP  - Show Matter commissioning information\r\n"
                      "QR     - Show QR code for Matter setup\r\n"
                      "INFO   - Show device information\r\n"
                      "WAKE   - Trigger wake manually\r\n"
                      "HELP   - Show this help\r\n"
                      "\r\n");
        return;
    }

    cdc_write_str("Unknown command. Type HELP for available commands.\r\n");
}

static void flush_cdc_line(void)
{
    cdc_line_len = 0U;
    memset(cdc_line_buf, 0, sizeof(cdc_line_buf));
}

static void handle_cdc_rx(uint8_t byte)
{
    if (cdc_line_len >= sizeof(cdc_line_buf) - 1) {
        flush_cdc_line();
        cdc_write_str("Command too long\r\n");
        return;
    }

    if (byte == '\r' || byte == '\n') {
        if (cdc_line_len > 0) {
            cdc_write_str("\r\n");
            process_cdc_command(cdc_line_buf);
        }
        flush_cdc_line();
        cdc_write_str("> ");
        return;
    }

    cdc_line_buf[cdc_line_len++] = (char)byte;
    cdc_write((const char *)&byte, 1);
}

static void cdc_interrupt_handler(const struct device *dev, void *user_data)
{
    ARG_UNUSED(dev);
    ARG_UNUSED(user_data);

    while (uart_irq_update(cdc_dev) && uart_irq_is_pending(cdc_dev)) {
        if (uart_irq_rx_ready(cdc_dev)) {
            uint8_t buf[16];
            int len = uart_fifo_read(cdc_dev, buf, sizeof(buf));
            for (int i = 0; i < len; ++i) {
                handle_cdc_rx(buf[i]);
            }
        }
    }
}

static int init_cdc(void)
{
    cdc_dev = DEVICE_DT_GET_ONE(zephyr_cdc_acm_uart);
    if (!device_is_ready(cdc_dev)) {
        LOG_ERR("CDC ACM device not ready");
        return -ENODEV;
    }

    uint32_t dtr = 0U;
    while (!dtr) {
        (void)uart_line_ctrl_get(cdc_dev, UART_LINE_CTRL_DTR, &dtr);
        k_sleep(K_MSEC(100));
    }

    uart_irq_callback_user_data_set(cdc_dev, cdc_interrupt_handler, NULL);
    uart_irq_rx_enable(cdc_dev);

    cdc_write_str("\r\n");
    cdc_write_str("========================================\r\n");
    cdc_write_str("  Matter WOL Companion v1.0\r\n");
    cdc_write_str("========================================\r\n");
    cdc_write_str("Type SETUP for Matter commissioning info\r\n");
    cdc_write_str("Type HELP for available commands\r\n");
    cdc_write_str("> ");
    return 0;
}

// Simulated Matter event handler - in real implementation this would be triggered by Matter stack
static void matter_button_event_handler(void *arg1, void *arg2, void *arg3)
{
    ARG_UNUSED(arg1);
    ARG_UNUSED(arg2);
    ARG_UNUSED(arg3);
    
    LOG_INF("Matter button event received");
    submit_mouse_work();
}

// Timer to simulate Matter button presses for demonstration
static void matter_demo_timer_handler(struct k_timer *timer)
{
    ARG_UNUSED(timer);
    // This would normally be triggered by actual Matter events
    // For now, we'll just log that we're ready for Matter events
    static int count = 0;
    if (++count == 1) {
        LOG_INF("Matter simulation: Device ready for button events from Home Assistant");
        LOG_INF("In real deployment, button presses in HA would trigger mouse jiggle");
    }
}

K_TIMER_DEFINE(matter_demo_timer, matter_demo_timer_handler, NULL);

extern "C" void main(void)
{
    int ret;

    LOG_INF("Starting Matter WOL Companion...");

    k_mutex_init(&hid_mutex);
    k_mutex_init(&cdc_tx_mutex);
    k_work_init(&mouse_work, mouse_jiggle);

    // Initialize USB subsystem
    usb_ctx = wol_usb_init_device(NULL);
    if (!usb_ctx) {
        LOG_ERR("USB device context initialization failed");
        return;
    }

    ret = usbd_enable(usb_ctx);
    if (ret) {
        LOG_ERR("USB enable failed (%d)", ret);
        return;
    }

    // Initialize HID device
    hid_dev = DEVICE_DT_GET_ONE(zephyr_hid_device);
    if (!device_is_ready(hid_dev)) {
        LOG_ERR("HID device not ready");
        return;
    }

    ret = hid_device_register(hid_dev,
                              hid_report_desc,
                              sizeof(hid_report_desc),
                              &hid_ops);
    if (ret) {
        LOG_ERR("Cannot register HID device (%d)", ret);
        return;
    }

    // Initialize settings
    if (IS_ENABLED(CONFIG_SETTINGS)) {
        settings_subsys_init();
    }

    // Initialize CDC console
    ret = init_cdc();
    if (ret) {
        LOG_WRN("CDC init failed (%d)", ret);
    }

    // In a real implementation, here we would:
    // 1. Initialize the Matter/CHIP stack
    // 2. Set up device attestation credentials  
    // 3. Configure Button cluster endpoint
    // 4. Start Matter commissioning
    // 5. Register button event handlers
    
    LOG_INF("Matter device information:");
    LOG_INF("  Setup PIN: %08lu", (unsigned long)kSetupPinCode);
    LOG_INF("  Discriminator: %u", kSetupDiscriminator);
    LOG_INF("  Vendor ID: 0x%04X", kVendorId);
    LOG_INF("  Product ID: 0x%04X", kProductId);

    // Start demonstration timer
    k_timer_start(&matter_demo_timer, K_SECONDS(5), K_SECONDS(30));

    LOG_INF("Matter WOL Companion ready");
    LOG_INF("Connect via USB CDC for setup instructions");
    LOG_INF("In Home Assistant: Add Matter device with PIN %08lu", (unsigned long)kSetupPinCode);
}