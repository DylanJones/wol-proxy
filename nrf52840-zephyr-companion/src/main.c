#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/sys/atomic.h>
#include <zephyr/sys/byteorder.h>
#include <zephyr/sys/util.h>
#include <zephyr/settings/settings.h>
#include <zephyr/usb/usbd.h>
#include <zephyr/usb/class/usbd_hid.h>
#include <zephyr/drivers/uart.h>
#include <zephyr/net/net_core.h>
#include <zephyr/net/net_if.h>
#include <zephyr/net/net_ip.h>
#include <zephyr/net/net_mgmt.h>
#include <zephyr/net/socket.h>

#include <ctype.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>

LOG_MODULE_REGISTER(wol_proxy, CONFIG_LOG_DEFAULT_LEVEL);

#include "usb_support.h"

#define SETTINGS_KEY "wol/port"
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
static atomic_t udp_port = ATOMIC_INIT(CONFIG_WOL_LISTEN_PORT);

static K_SEM_DEFINE(net_ready, 0, 1);
static struct net_mgmt_event_callback net_cb;
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

static void udp_trigger(void)
{
    submit_mouse_work();
}

static void net_event_handler(struct net_mgmt_event_callback *cb, uint64_t mgmt_event,
                              struct net_if *iface)
{
    ARG_UNUSED(cb);
    ARG_UNUSED(iface);

    if (mgmt_event == NET_EVENT_L4_CONNECTED || mgmt_event == NET_EVENT_IF_UP) {
        k_sem_give(&net_ready);
    }
}

static int wol_settings_set(const char *key, size_t len, settings_read_cb read_cb, void *cb_arg)
{
    if (!key) {
        return -EINVAL;
    }

    if (strcmp(key, "port") != 0) {
        return -ENOENT;
    }

    if (len != sizeof(uint16_t)) {
        return -EINVAL;
    }

    uint16_t stored;
    ssize_t rc = read_cb(cb_arg, &stored, sizeof(stored));
    if (rc < 0) {
        return (int)rc;
    }

    uint16_t port = sys_le16_to_cpu(stored);
    if (port == 0) {
        port = CONFIG_WOL_LISTEN_PORT;
    }

    atomic_set(&udp_port, port);
    LOG_INF("Loaded persisted port %u", port);
    return 0;
}

static int wol_settings_export(int (*export_func)(const char *name, const void *value, size_t val_len))
{
    uint16_t port = (uint16_t)atomic_get(&udp_port);
    uint16_t stored = sys_cpu_to_le16(port);
    return export_func(SETTINGS_KEY, &stored, sizeof(stored));
}

SETTINGS_STATIC_HANDLER_DEFINE(wol, "wol", NULL, wol_settings_set, NULL, wol_settings_export);

static void persist_port(uint16_t port)
{
    uint16_t stored = sys_cpu_to_le16(port);
    int err = settings_save_one(SETTINGS_KEY, &stored, sizeof(stored));
    if (err) {
        LOG_WRN("Failed to persist port (%d)", err);
    }
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

static void cdc_show_port(void)
{
    char line[32];
    uint16_t port = (uint16_t)atomic_get(&udp_port);
    int n = snprintk(line, sizeof(line), "PORT=%u\r\n", port);
    if (n > 0) {
        cdc_write(line, (size_t)n);
    }
}

static void cdc_apply_port(uint16_t port)
{
    atomic_set(&udp_port, port);
    persist_port(port);
    LOG_INF("Port updated to %u", port);
    cdc_write_str(CDC_OK);
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
        cdc_show_port();
        return;
    }

    if (match_keyword(line, "PORT")) {
        line += 4;
        while (isspace((unsigned char)*line)) {
            ++line;
        }
        if (*line == '\0') {
            cdc_write_str(CDC_ERR);
            return;
        }

        char *endptr = NULL;
        long value = strtol(line, &endptr, 10);
        if (endptr == line || value < 1 || value > 65535) {
            cdc_write_str(CDC_ERR);
            return;
        }
        cdc_apply_port((uint16_t)value);
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

static void udp_listener(void *, void *, void *)
{
    uint16_t active_port = 0;
    int sock = -1;
    struct sockaddr_in6 bind_addr = {0};
    uint8_t rx_buf[256];

    bind_addr.sin6_family = AF_INET6;
    bind_addr.sin6_addr = in6addr_any;

    (void)k_sem_take(&net_ready, K_FOREVER);
    LOG_INF("Network ready; starting UDP listener");

    while (true) {
        uint16_t desired = (uint16_t)atomic_get(&udp_port);
        if (desired == 0U) {
            desired = CONFIG_WOL_LISTEN_PORT;
        }

        if (sock < 0 || desired != active_port) {
            if (sock >= 0) {
                zsock_close(sock);
            }
            sock = zsock_socket(AF_INET6, SOCK_DGRAM, IPPROTO_UDP);
            if (sock < 0) {
                LOG_ERR("Failed to create UDP socket (%d)", errno);
                k_sleep(K_SECONDS(1));
                continue;
            }
            bind_addr.sin6_port = htons(desired);
            if (zsock_bind(sock, (struct sockaddr *)&bind_addr, sizeof(bind_addr)) < 0) {
                LOG_ERR("Failed to bind UDP socket to %u (%d)", desired, errno);
                zsock_close(sock);
                sock = -1;
                k_sleep(K_SECONDS(1));
                continue;
            }
            active_port = desired;
            LOG_INF("Listening on UDP port %u", active_port);
        }

        int received = zsock_recv(sock, rx_buf, sizeof(rx_buf), 0);
        if (received < 0) {
            if (errno == EINTR) {
                continue;
            }
            LOG_WRN("UDP recv error (%d)", errno);
            k_sleep(K_MSEC(10));
            continue;
        }

        if (received > 0) {
            LOG_DBG("Received %d bytes", received);
            udp_trigger();
        }
    }
}

K_THREAD_STACK_DEFINE(udp_listener_stack, 4096);
static struct k_thread udp_thread_data;

static void start_udp_listener(void)
{
    k_thread_create(&udp_thread_data, udp_listener_stack, K_THREAD_STACK_SIZEOF(udp_listener_stack),
                    udp_listener, NULL, NULL, NULL, K_PRIO_COOP(8), 0, K_NO_WAIT);
    k_thread_name_set(&udp_thread_data, "udp_listener");
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

    if (IS_ENABLED(CONFIG_SETTINGS)) {
        settings_subsys_init();
        settings_load_subtree("wol");
    }

    ret = init_cdc();
    if (ret) {
        LOG_WRN("CDC init failed (%d)", ret);
    }

    net_mgmt_init_event_callback(&net_cb, net_event_handler,
                                 NET_EVENT_L4_CONNECTED | NET_EVENT_IF_UP);
    net_mgmt_add_event_callback(&net_cb);

    struct net_if *iface = net_if_get_default();
    if (iface && net_if_is_up(iface)) {
        k_sem_give(&net_ready);
    }

    start_udp_listener();
    LOG_INF("nRF52840 Zephyr companion ready");
}
