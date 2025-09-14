#![no_std]
#![no_main]

// Modern ESP32-S2 firmware with esp-hal 1.0, esp-wifi 0.15, and Embassy 0.9/0.5.
// Provides a composite USB device (HID mouse + CDC). It connects to Wi‑Fi,
// listens on a configurable UDP port (default 3389), and upon receiving any
// datagram, emits a tiny mouse movement to wake the host.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::sync::atomic::{AtomicU16, Ordering};

use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config as NetConfig, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embassy_usb::{
    self as eusb,
    class::{cdc_acm, hid},
};
use esp_alloc as _;
use esp_backtrace as _; // panic handler
use esp_hal::otg_fs::{asynch as usb_async, Usb};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use static_cell::StaticCell;
use usbd_hid::descriptor::{MouseReport, SerializedDescriptor};

mod config;
mod udp_sig;

// Default UDP port; can be changed over runtime config (USB CDC)
const DEFAULT_PORT: u16 = 3389;
static UDP_PORT: AtomicU16 = AtomicU16::new(DEFAULT_PORT);

// Channel from UDP listener to USB task to request a jiggle
static WAKE_CH: Channel<CriticalSectionRawMutex, (), 4> = Channel::new();
type UsbDrv = usb_async::Driver<'static>;
// StaticCell allocations to avoid &mut to mutable statics
static EP_OUT_BUF: StaticCell<[u8; 256]> = StaticCell::new();
static USB_DESC: StaticCell<[u8; 256]> = StaticCell::new();
static USB_BOS: StaticCell<[u8; 64]> = StaticCell::new();
static USB_MSOS: StaticCell<[u8; 64]> = StaticCell::new();
static USB_CTRL: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<cdc_acm::State<'static>> = StaticCell::new();
static HID_STATE: StaticCell<hid::State<'static>> = StaticCell::new();
static NET_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
static WIFI_CTRL: StaticCell<esp_wifi::EspWifiController<'static>> = StaticCell::new();

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, esp_wifi::wifi::WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn udp_listener_task(stack: Stack<'static>) {
    // Wait for network config
    stack.wait_config_up().await;

    // UDP socket buffers/metadata
    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 0];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);

    let mut pkt = [0u8; 256];
    loop {
        let port = UDP_PORT.load(Ordering::Relaxed);
        let _ = socket.bind(embassy_net::IpListenEndpoint { addr: None, port });

        if let Ok((n, _meta)) = socket.recv_from(&mut pkt).await {
            if udp_sig::packet_means_wake(&pkt[..n]) {
                let _ = WAKE_CH.try_send(());
            }
        }
    }
}

#[embassy_executor::task]
async fn usb_device_task(mut dev: eusb::UsbDevice<'static, UsbDrv>) {
    dev.run().await;
}

#[embassy_executor::task]
async fn usb_cdc_task(
    mut tx: cdc_acm::Sender<'static, UsbDrv>,
    mut rx: cdc_acm::Receiver<'static, UsbDrv>,
) {
    let mut buf = [0u8; 128];
    loop {
        // Wait for connection
        tx.wait_connection().await;
        rx.wait_connection().await;

        // Read a single packet (<= max_packet_size) then process
        match rx.read_packet(&mut buf).await {
            Ok(n) if n > 0 => {
                let _ = tx.write_packet(&buf[..n]).await; // echo
                handle_serial_line(&buf[..n], &mut tx).await;
            }
            _ => Timer::after_millis(1).await,
        }
    }
}

#[embassy_executor::task]
async fn usb_hid_task(mut hid: hid::HidWriter<'static, UsbDrv, 64>) {
    loop {
        let _ = WAKE_CH.receive().await;
        jiggle_mouse(&mut hid).await;
    }
}

async fn handle_serial_line(bytes: &[u8], serial: &mut cdc_acm::Sender<'static, UsbDrv>) {
    if let Ok(s) = core::str::from_utf8(bytes) {
        let s = s.trim();
        if s.is_empty() {
            return;
        }
        if s.starts_with('{') {
            if let Ok(cmd) = config::parse_json_config(s) {
                apply_config(cmd, serial).await;
                return;
            }
        }
        if let Some(cmd) = config::parse_plain_config(s) {
            apply_config(cmd, serial).await;
            return;
        }
        let _ = serial.write_packet(b"ERR: invalid config\r\n").await;
    }
}

async fn apply_config(cmd: config::ConfigCommand, serial: &mut cdc_acm::Sender<'static, UsbDrv>) {
    match cmd {
        config::ConfigCommand::SetWifi { ssid, pass } => {
            if config::save_wifi(&ssid, &pass).is_ok() {
                let _ = serial.write_packet(b"OK WIFI\r\n").await;
            } else {
                let _ = serial.write_packet(b"ERR WIFI\r\n").await;
            }
        }
        config::ConfigCommand::SetPort { port } => {
            UDP_PORT.store(port, Ordering::Relaxed);
            let _ = config::save_port(port);
            let _ = serial.write_packet(b"OK PORT\r\n").await;
        }
        config::ConfigCommand::Show => {
            let (ssid, _pass) = config::load_wifi().unwrap_or((String::new(), String::new()));
            let port = UDP_PORT.load(Ordering::Relaxed);
            let _ = serial
                .write_packet(format!("SSID={ssid}; PORT={port}\r\n").as_bytes())
                .await;
        }
    }
}

async fn jiggle_mouse(hid: &mut hid::HidWriter<'static, UsbDrv, 64>) {
    let r1 = MouseReport {
        buttons: 0,
        x: 1,
        y: 0,
        wheel: 0,
        pan: 0,
    };
    let r2 = MouseReport {
        buttons: 0,
        x: -1,
        y: 0,
        wheel: 0,
        pan: 0,
    };
    let _ = hid.write_serialize(&r1).await;
    let _ = hid.write_serialize(&r2).await;
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // Chip + clocks
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Embassy time driver
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // USB (Embassy): driver + classes + device
    let usb_peri = Usb::new(peripherals.USB0, peripherals.GPIO20, peripherals.GPIO19);
    let driver = usb_async::Driver::new(
        usb_peri,
        EP_OUT_BUF.init([0; 256]),
        usb_async::Config::default(),
    );

    let mut cfg = eusb::Config::new(0x1209, 0x0001);
    cfg.manufacturer = Some("WOL-Proxy");
    cfg.product = Some("ESP32-S2 Wake Mouse");
    cfg.serial_number = Some("ESP32S2");
    cfg.max_power = 100;
    cfg.max_packet_size_0 = 64;

    let mut builder = eusb::Builder::new(
        driver,
        cfg,
        USB_DESC.init([0; 256]),
        USB_BOS.init([0; 64]),
        USB_MSOS.init([0; 64]),
        USB_CTRL.init([0; 64]),
    );

    // CDC-ACM
    let cdc = cdc_acm::CdcAcmClass::new(&mut builder, CDC_STATE.init(cdc_acm::State::new()), 64);
    let (cdc_tx, cdc_rx) = cdc.split();

    // HID writer (Mouse)
    let hid_cfg = hid::Config {
        report_descriptor: MouseReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 64,
    };
    let hid_writer =
        hid::HidWriter::<UsbDrv, 64>::new(&mut builder, HID_STATE.init(hid::State::new()), hid_cfg);

    let usb_dev = builder.build();

    // Load port if provided earlier
    if let Some(p) = config::load_port().ok().flatten() {
        UDP_PORT.store(p, Ordering::Relaxed);
    }

    // Spawn USB device + classes tasks (device must run concurrently)
    spawner.spawn(usb_device_task(usb_dev)).ok();
    spawner.spawn(usb_cdc_task(cdc_tx, cdc_rx)).ok();
    spawner.spawn(usb_hid_task(hid_writer)).ok();

    // Wait for Wi‑Fi credentials via USB CDC if none pre-loaded
    let (ssid, pass) = loop {
        if let Ok((s, p)) = config::load_wifi() {
            if !s.is_empty() {
                break (s, p);
            }
        }
        println!("No Wi‑Fi configured yet. Over USB CDC:\r\n  WIFI <ssid> <pass>\r\n  PORT <num>\r\n  SHOW\r\n");
        Timer::after_secs(1).await;
    };

    // Wi‑Fi init + network stack
    let rng = Rng::new(peripherals.RNG);
    let ctrl = esp_wifi::init(timg0.timer1, rng).unwrap();
    let ctrl_ref: &'static mut esp_wifi::EspWifiController<'static> = WIFI_CTRL.init(ctrl);
    let (mut wifi, ifs) = esp_wifi::wifi::new(ctrl_ref, peripherals.WIFI).unwrap();

    use esp_wifi::wifi::{ClientConfiguration, Configuration};
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid,
        password: pass,
        ..Default::default()
    }))
    .unwrap();
    wifi.start_async().await.unwrap();
    wifi.connect_async().await.unwrap();

    // Network stack with DHCPv4
    let seed = 0x1234_5678_9ABC_DEF0u64;
    let (stack, runner) = embassy_net::new(
        ifs.sta,
        NetConfig::dhcpv4(Default::default()),
        NET_RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(udp_listener_task(stack)).ok();

    // Keep alive
    loop {
        Timer::after_secs(3600).await;
    }
}
