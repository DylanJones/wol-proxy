#include "matter_support.h"

#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/sys/util.h>

#include <errno.h>

#if defined(CONFIG_CHIP)

#include <app-common/zap-generated/attributes/Accessors.h>
#include <app-common/zap-generated/ids/Attributes.h>
#include <app-common/zap-generated/ids/Clusters.h>
#include <app/server/Server.h>
#include <app/util/attribute-storage.h>
#include <lib/core/CHIPError.h>
#include <lib/support/CHIPMem.h>
#include <lib/support/logging/CHIPLogging.h>
#include <platform/CHIPDeviceLayer.h>
#include <setup_payload/ManualSetupPayloadGenerator.h>
#include <setup_payload/OnboardingCodesUtil.h>
#include <setup_payload/QRCodeSetupPayloadGenerator.h>
#include <setup_payload/SetupPayload.h>

#include <string>

LOG_MODULE_REGISTER(matter_support, CONFIG_LOG_DEFAULT_LEVEL);

using namespace chip;
using namespace chip::app;
using namespace chip::DeviceLayer;

namespace {
constexpr EndpointId kMatterEndpointId = 1;

K_MUTEX_DEFINE(s_code_mutex);
bool s_codes_ready = false;
std::string s_manual_code;
std::string s_qr_code;
matter_wake_callback_t s_wake_callback = nullptr;

CHIP_ERROR GenerateOnboardingCodesLocked()
{
    SetupPayload payload;
    uint16_t vendor = 0;
    uint16_t product = 0;
    uint16_t discriminator = 0;
    uint32_t passcode = 0;

    if (ConfigurationMgr().GetVendorId(vendor) == CHIP_NO_ERROR) {
        payload.vendorID = vendor;
    }
    if (ConfigurationMgr().GetProductId(product) == CHIP_NO_ERROR) {
        payload.productID = product;
    }
    if (ConfigurationMgr().GetSetupDiscriminator(discriminator) == CHIP_NO_ERROR) {
        payload.discriminator.SetLongValue(discriminator);
    }
    if (ConfigurationMgr().GetSetupPasscode(passcode) == CHIP_NO_ERROR) {
        payload.setUpPINCode = passcode;
    }
    payload.rendezvousInformation.SetValue(RendezvousInformationFlags(RendezvousInformationFlag::kBLE));

    CHIP_ERROR err = CHIP_NO_ERROR;

    PayloadContents contents(payload);
    ManualSetupPayloadGenerator manual_generator(contents);
    if ((err = manual_generator.payloadDecimalStringRepresentation(s_manual_code)) != CHIP_NO_ERROR) {
        return err;
    }

    QRCodeSetupPayloadGenerator qr_generator(payload);
    if ((err = qr_generator.payloadBase38RepresentationWithAutoTLVBuffer(s_qr_code)) != CHIP_NO_ERROR) {
        return err;
    }

    return CHIP_NO_ERROR;
}

CHIP_ERROR EnsureCodesGenerated()
{
    k_mutex_lock(&s_code_mutex, K_FOREVER);
    CHIP_ERROR err = CHIP_NO_ERROR;
    if (!s_codes_ready) {
        err = GenerateOnboardingCodesLocked();
        if (err == CHIP_NO_ERROR) {
            s_codes_ready = true;
        }
    }
    k_mutex_unlock(&s_code_mutex);
    return err;
}

} // namespace

extern "C" int matter_support_init(matter_wake_callback_t wake_cb)
{
    s_wake_callback = wake_cb;

    CHIP_ERROR err = Platform::MemoryInit();
    if (err != CHIP_NO_ERROR) {
        LOG_ERR("MemoryInit failed: %" CHIP_ERROR_FORMAT, err.Format());
        return -ENOMEM;
    }

    err = PlatformMgr().InitChipStack();
    if (err != CHIP_NO_ERROR) {
        LOG_ERR("InitChipStack failed: %" CHIP_ERROR_FORMAT, err.Format());
        return -EIO;
    }

    ConfigurationMgr().LogDeviceConfig();

    err = Server::GetInstance().Init();
    if (err != CHIP_NO_ERROR) {
        LOG_ERR("Matter server init failed: %" CHIP_ERROR_FORMAT, err.Format());
        return -EIO;
    }

    err = PlatformMgr().StartEventLoopTask();
    if (err != CHIP_NO_ERROR) {
        LOG_ERR("Failed to start Matter event loop: %" CHIP_ERROR_FORMAT, err.Format());
        return -EIO;
    }

    err = EnsureCodesGenerated();
    if (err != CHIP_NO_ERROR) {
        LOG_ERR("Failed to generate onboarding codes: %" CHIP_ERROR_FORMAT, err.Format());
        return -EIO;
    }

    return 0;
}

extern "C" void matter_support_log_onboarding(void)
{
    if (EnsureCodesGenerated() != CHIP_NO_ERROR) {
        LOG_WRN("Matter onboarding codes are unavailable");
        return;
    }

    PrintOnboardingCodes(RendezvousInformationFlags::kBLE);
    LOG_INF("Matter manual code: %s", log_strdup(s_manual_code.c_str()));
    LOG_INF("Matter QR code: %s", log_strdup(s_qr_code.c_str()));
}

extern "C" const char *matter_support_manual_code(void)
{
    if (EnsureCodesGenerated() != CHIP_NO_ERROR) {
        return nullptr;
    }
    return s_manual_code.c_str();
}

extern "C" const char *matter_support_qr_code(void)
{
    if (EnsureCodesGenerated() != CHIP_NO_ERROR) {
        return nullptr;
    }
    return s_qr_code.c_str();
}

void MatterPostAttributeChangeCallback(const chip::app::ConcreteAttributePath & attributePath, uint8_t type,
                                       uint16_t size, uint8_t * value)
{
    ARG_UNUSED(type);
    ARG_UNUSED(size);

    if (!value) {
        return;
    }

    if (attributePath.mClusterId == chip::app::Clusters::OnOff::Id &&
        attributePath.mAttributeId == chip::app::Clusters::OnOff::Attributes::OnOff::Id) {
        LOG_INF("Matter OnOff attribute updated: %u", *value);
        if (*value != 0U && s_wake_callback) {
            s_wake_callback();
            auto status = chip::app::Clusters::OnOff::Attributes::OnOff::Set(kMatterEndpointId, false);
            if (status != chip::Protocols::InteractionModel::Status::Success) {
                LOG_WRN("Failed to reset OnOff attribute (%u)", static_cast<unsigned>(status));
            }
        }
    }
}

void emberAfOnOffClusterInitCallback(chip::EndpointId endpoint)
{
    ARG_UNUSED(endpoint);
    auto status = chip::app::Clusters::OnOff::Attributes::OnOff::Set(kMatterEndpointId, false);
    if (status != chip::Protocols::InteractionModel::Status::Success) {
        LOG_WRN("Failed to initialize OnOff attribute (%u)", static_cast<unsigned>(status));
    }
}

#else

extern "C" int matter_support_init(matter_wake_callback_t wake_cb)
{
    ARG_UNUSED(wake_cb);
    LOG_WRN("Matter support not enabled; set CONFIG_CHIP to enable provisioning");
    return -ENOTSUP;
}

extern "C" void matter_support_log_onboarding(void)
{
    LOG_WRN("Matter support not enabled");
}

extern "C" const char *matter_support_manual_code(void)
{
    return nullptr;
}

extern "C" const char *matter_support_qr_code(void)
{
    return nullptr;
}

#endif
