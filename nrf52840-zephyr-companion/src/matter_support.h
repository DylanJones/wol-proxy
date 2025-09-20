#ifndef MATTER_SUPPORT_H
#define MATTER_SUPPORT_H

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*matter_wake_callback_t)(void);

int matter_support_init(matter_wake_callback_t wake_cb);
void matter_support_log_onboarding(void);
const char *matter_support_manual_code(void);
const char *matter_support_qr_code(void);

#ifdef __cplusplus
}
#endif

#endif /* MATTER_SUPPORT_H */
