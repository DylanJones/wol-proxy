#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/sys/util.h>
#include <zephyr/usb/usbd.h>
#include <zephyr/usb/class/usbd_hid.h>
#include <zephyr/drivers/uart.h>

#include <ctype.h>
#include <errno.h>
#include <string.h>

LOG_MODULE_REGISTER(wol_proxy, CONFIG_LOG_DEFAULT_LEVEL);

#include "usb_support.h"
#include "matter_support.h"

#define CDC_PROMPT "> "
#define CDC_OK "OK\r\n"
#define CDC_ERR "ERR\r\n"

static const uint8_t hid_report_desc[] = HID_MOUSE_REPORT_DESC(3);

struct mouse_report {
    uint8_t buttons;
    int8_t x;
    int8_t y;
    int8_t wheel;
} __packed;

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

static void submit_mouse_work(void);

static void hid_iface_ready(const struct device *dev, const bool ready)
{
    ARG_UNUSED(dev);
    hid_ready = ready;
}

static struct hid_device_ops hid_ops = {
    .iface_ready = hid_iface_ready,
};

static void mouse_jiggle(struct k_work *work)
{
    ARG_UNUSED(work);

    if (!hid_dev || !hid_ready) {
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
}

static void submit_mouse_work(void)
{
    k_work_submit(&mouse_work);
}

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

static void cdc_show_codes(void)
{
    const char *manual = matter_support_manual_code();
    const char *qr = matter_support_qr_code();

    if (manual && manual[0] != '\0') {
        cdc_write_str("MANUAL=");
        cdc_write_str(manual);
        cdc_write_str("\r\n");
    } else {
        cdc_write_str("MANUAL=UNAVAILABLE\r\n");
    }

    if (qr && qr[0] != '\0') {
        cdc_write_str("QR=");
        cdc_write_str(qr);
        cdc_write_str("\r\n");
    } else {
        cdc_write_str("QR=UNAVAILABLE\r\n");
    }
}

static void cdc_print_help(void)
{
    cdc_write_str("SHOW - display Matter onboarding codes\r\n");
    cdc_write_str("HELP - display this message\r\n");
}

static bool match_keyword(const char *buf, const char *keyword)
{
    while (*keyword && *buf) {
        if (toupper((unsigned char)*buf) != toupper((unsigned char)*keyword)) {
            return false;
        }
        ++buf;
        ++keyword;
    }
    return *keyword == '\0';
}

static void process_cdc_command(const char *line)
{
    while (isspace((unsigned char)*line)) {
        ++line;
    }
    if (*line == '\0') {
        return;
    }

    if (match_keyword(line, "SHOW")) {
        cdc_show_codes();
        cdc_write_str(CDC_OK);
        return;
    }

    if (match_keyword(line, "HELP")) {
        cdc_print_help();
        cdc_write_str(CDC_OK);
        return;
    }

    cdc_write_str(CDC_ERR);
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
        cdc_write_str(CDC_ERR);
        return;
    }

    if (byte == '\r' || byte == '\n') {
        if (cdc_line_len > 0) {
            cdc_write_str("\r\n");
            process_cdc_command(cdc_line_buf);
        }
        flush_cdc_line();
        cdc_write_str(CDC_PROMPT);
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

    cdc_write_str("WOL Companion Ready\r\n");
    cdc_write_str(CDC_PROMPT);
    return 0;
}

void main(void)
{
    int ret;

    k_mutex_init(&hid_mutex);
    k_mutex_init(&cdc_tx_mutex);
    k_work_init(&mouse_work, mouse_jiggle);

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

    ret = init_cdc();
    if (ret) {
        LOG_WRN("CDC init failed (%d)", ret);
    }

    ret = matter_support_init(submit_mouse_work);
    if (ret) {
        LOG_ERR("Matter initialization failed (%d)", ret);
        cdc_write_str("Matter init failed\r\n");
    } else {
        matter_support_log_onboarding();
        cdc_write_str("Type SHOW for onboarding codes\r\n");
    }

    LOG_INF("nRF52840 Zephyr companion ready");
}
