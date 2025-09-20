#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::config::{Config, LfclkSource, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::usb::{self, Driver};
use embassy_usb::Builder as UsbBuilder;
use static_cell::StaticCell;
use embassy_nrf::{bind_interrupts, peripherals};
use embassy_time::{Timer, Duration};
use panic_halt as _;
use cortex_m::asm::delay;

// Minimal bring-up: mimic xiao-embassy-hello

// Bind required interrupt for USBD at module scope
bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Use external 32.768 kHz crystal for LFCLK
    let mut cfg = Config::default();
    cfg.lfclk_source = LfclkSource::ExternalXtal;
    cfg.hfclk_source = HfclkSource::ExternalXtal; // stable HFCLK for USBD
    let p = embassy_nrf::init(cfg);

    // Configure pins (start HIGH = off). XIAO nRF52840 RGB pins (active-low):
    // Red: P0.26, Blue: P0.06, Green: P0.30
    let mut red = Output::new(p.P0_26, Level::High, OutputDrive::Standard);
    let mut blue = Output::new(p.P0_06, Level::High, OutputDrive::Standard);
    let mut green = Output::new(p.P0_30, Level::High, OutputDrive::Standard);


    // test if resetting
    red.set_low();
    blue.set_low();
    delay(CYCLES_150MS);
    blue.set_high();
    red.set_high();
    delay(CYCLES_150MS);

    // Spawn three blinks like the hello example
    #[embassy_executor::task(pool_size = 3)]
    async fn blink(mut led: Output<'static>, on: Duration, off: Duration) {
        loop {
            led.set_low();
            Timer::after(on).await;
            led.set_high();
            Timer::after(off).await;
        }
    }

    let _ = spawner.spawn(blink(red, Duration::from_millis(100), Duration::from_millis(900)));
    let _ = spawner.spawn(blink(green, Duration::from_millis(200), Duration::from_millis(800)));
    let _ = spawner.spawn(blink(blue, Duration::from_millis(300), Duration::from_millis(700)));

    loop { Timer::after(Duration::from_secs(1)).await; }
}
