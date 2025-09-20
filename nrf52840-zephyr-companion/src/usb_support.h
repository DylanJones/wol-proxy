#ifndef WOL_USB_SUPPORT_H_
#define WOL_USB_SUPPORT_H_

#include <zephyr/usb/usbd.h>

struct usbd_context *wol_usb_init_device(usbd_msg_cb_t cb);

#endif /* WOL_USB_SUPPORT_H_ */
