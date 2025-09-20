#include "usb_support.h"

#include <zephyr/usb/bos.h>
#include <zephyr/logging/log.h>

LOG_MODULE_REGISTER(wol_usb_support, CONFIG_LOG_DEFAULT_LEVEL);

#define WOL_USB_VID 0x1209
#define WOL_USB_PID 0x0002
#define WOL_USB_MANUFACTURER "WOL-Proxy"
#define WOL_USB_PRODUCT "nRF52840 Zephyr Wake Mouse"

static const char *const class_blocklist[] = {
    NULL,
};

USBD_DEVICE_DEFINE(wol_usbd,
                   DEVICE_DT_GET(DT_NODELABEL(zephyr_udc0)),
                   WOL_USB_VID, WOL_USB_PID);

USBD_DESC_LANG_DEFINE(wol_lang);
USBD_DESC_MANUFACTURER_DEFINE(wol_mfr, WOL_USB_MANUFACTURER);
USBD_DESC_PRODUCT_DEFINE(wol_product, WOL_USB_PRODUCT);
IF_ENABLED(CONFIG_HWINFO, (USBD_DESC_SERIAL_NUMBER_DEFINE(wol_sn)));

USBD_DESC_CONFIG_DEFINE(fs_cfg_desc, "Wake Config");
#if USBD_SUPPORTS_HIGH_SPEED
USBD_DESC_CONFIG_DEFINE(hs_cfg_desc, "Wake Config HS");
#endif

static const uint8_t wol_attributes = USB_SCD_REMOTE_WAKEUP;

USBD_CONFIGURATION_DEFINE(wol_fs_config,
                          wol_attributes,
                          100,
                          &fs_cfg_desc);
#if USBD_SUPPORTS_HIGH_SPEED
USBD_CONFIGURATION_DEFINE(wol_hs_config,
                          wol_attributes,
                          100,
                          &hs_cfg_desc);
#endif

static void fix_code_triple(struct usbd_context *ctx, enum usbd_speed speed)
{
    if (IS_ENABLED(CONFIG_USBD_CDC_ACM_CLASS) ||
        IS_ENABLED(CONFIG_USBD_CDC_ECM_CLASS) ||
        IS_ENABLED(CONFIG_USBD_CDC_NCM_CLASS) ||
        IS_ENABLED(CONFIG_USBD_MIDI2_CLASS) ||
        IS_ENABLED(CONFIG_USBD_AUDIO2_CLASS) ||
        IS_ENABLED(CONFIG_USBD_VIDEO_CLASS)) {
        usbd_device_set_code_triple(ctx, speed,
                                    USB_BCC_MISCELLANEOUS, 0x02, 0x01);
    } else {
        usbd_device_set_code_triple(ctx, speed, 0, 0, 0);
    }
}

static struct usbd_context *setup_device(usbd_msg_cb_t cb)
{
    int err;

    err = usbd_add_descriptor(&wol_usbd, &wol_lang);
    if (err) {
        LOG_ERR("Failed to add language descriptor (%d)", err);
        return NULL;
    }

    err = usbd_add_descriptor(&wol_usbd, &wol_mfr);
    if (err) {
        LOG_ERR("Failed to add manufacturer descriptor (%d)", err);
        return NULL;
    }

    err = usbd_add_descriptor(&wol_usbd, &wol_product);
    if (err) {
        LOG_ERR("Failed to add product descriptor (%d)", err);
        return NULL;
    }

    IF_ENABLED(CONFIG_HWINFO, (
        err = usbd_add_descriptor(&wol_usbd, &wol_sn);
        if (err) {
            LOG_ERR("Failed to add serial descriptor (%d)", err);
            return NULL;
        }
    ));

#if USBD_SUPPORTS_HIGH_SPEED
    if (usbd_caps_speed(&wol_usbd) == USBD_SPEED_HS) {
        err = usbd_add_configuration(&wol_usbd, USBD_SPEED_HS,
                                     &wol_hs_config);
        if (err) {
            LOG_ERR("Failed to add high-speed configuration (%d)", err);
            return NULL;
        }

        err = usbd_register_all_classes(&wol_usbd, USBD_SPEED_HS, 1,
                                        class_blocklist);
        if (err) {
            LOG_ERR("Failed to register HS classes (%d)", err);
            return NULL;
        }

        fix_code_triple(&wol_usbd, USBD_SPEED_HS);
    }
#endif

    err = usbd_add_configuration(&wol_usbd, USBD_SPEED_FS,
                                 &wol_fs_config);
    if (err) {
        LOG_ERR("Failed to add full-speed configuration (%d)", err);
        return NULL;
    }

    err = usbd_register_all_classes(&wol_usbd, USBD_SPEED_FS, 1,
                                    class_blocklist);
    if (err) {
        LOG_ERR("Failed to register FS classes (%d)", err);
        return NULL;
    }

    fix_code_triple(&wol_usbd, USBD_SPEED_FS);
    usbd_self_powered(&wol_usbd, false);

    if (cb) {
        err = usbd_msg_register_cb(&wol_usbd, cb);
        if (err) {
            LOG_ERR("Failed to register message callback (%d)", err);
            return NULL;
        }
    }

    return &wol_usbd;
}

struct usbd_context *wol_usb_init_device(usbd_msg_cb_t cb)
{
    struct usbd_context *ctx = setup_device(cb);
    if (!ctx) {
        return NULL;
    }

    int err = usbd_init(ctx);
    if (err) {
        LOG_ERR("Failed to initialize USB device (%d)", err);
        return NULL;
    }

    return ctx;
}
