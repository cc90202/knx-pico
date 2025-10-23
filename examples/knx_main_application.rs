#![no_std]
#![no_main]
#![allow(dead_code)]

mod common;

use common::utility::*;
use cyw43::Control;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use panic_persist as _;
use static_cell::StaticCell;

// Conditional imports based on logger choice
#[cfg(feature = "usb-logger")]
use embassy_rp::peripherals::USB;
#[cfg(feature = "usb-logger")]
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};

// defmt-rtt is always needed because dependencies use defmt internally
use defmt_rtt as _;

// Network stack imports
use embassy_net::{Config, StackResources};

// Import unified logging macro from knx_pico crate
use knx_pico::pico_log;

// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"KNX-RS"),
    embassy_rp::binary_info::rp_program_description!(
        c"KNX protocol implementation for Raspberry Pico 2 W"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

// Interrupt handlers
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[cfg(feature = "usb-logger")]
bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

/// Shared structure to pass the CYW43 controller between Embassy tasks
#[derive(Clone, Copy)]
#[allow(missing_debug_implementations)]
pub struct SharedControl(&'static Mutex<CriticalSectionRawMutex, Control<'static>>);

/// Main entry point for Embassy executor
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Start appropriate logger based on active feature
    #[cfg(feature = "usb-logger")]
    {
        let driver = Driver::new(p.USB, UsbIrqs);
        spawner.must_spawn(logger_task(driver));
    }

    if let Some(panic_message) = panic_persist::get_panic_message_utf8() {
        pico_log!(error, "{}", panic_message);
        loop {
            Timer::after_secs(5).await;
        }
    }

    // Firmware files for the CYW43xxx WiFi chip
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    spawner.must_spawn(cyw43_task(runner));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Configure network stack with DHCP
    let config = Config::dhcpv4(Default::default());

    // Generate random seed for network stack
    let seed: u64 = RoscRng.next_u64();
    pico_log!(info, "Random seed: {}", seed);

    // Initialize network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    let shared_control = SharedControl(&*{
        static CONTROL: StaticCell<Mutex<CriticalSectionRawMutex, Control<'static>>> =
            StaticCell::new();
        CONTROL.init(Mutex::new(control))
    });

    spawner.must_spawn(blink_task(shared_control));

    // WiFi connection configuration from configuration.rs
    let wifi_ssid = get_ssid();
    let wifi_password = get_wifi_password();

    pico_log!(info, "Connecting to WiFi network: {}", wifi_ssid);

    // Join WiFi network
    loop {
        {
            let mut control = shared_control.0.lock().await;
            match control.join(wifi_ssid, cyw43::JoinOptions::new(wifi_password.as_bytes())).await {
                Ok(_) => {
                    pico_log!(info, "WiFi connected successfully!");
                    break;
                }
                Err(e) => {
                    pico_log!(error, "WiFi connection failed: status={}, retrying in 5s...", e.status);
                }
            }
        }
        Timer::after(Duration::from_secs(5)).await;
    }

    // Wait for DHCP to assign IP address
    pico_log!(info, "Waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }

    if let Some(config) = stack.config_v4() {
        pico_log!(info, "IP Address: {}", config.address);
        pico_log!(info, "Gateway: {:?}", config.gateway);
    }

    pico_log!(info, "✓ KNX-RS initialized and ready!");
    pico_log!(info, "System running in idle mode");
    pico_log!(info, "To test KNX functionality, run: cargo run --example knx_sniffer");

    // Main application loop - heartbeat only
    loop {
        Timer::after(Duration::from_secs(30)).await;
        pico_log!(info, "Heartbeat: system alive");
    }
}

// Background tasks
#[cfg(feature = "usb-logger")]
#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn blink_task(control: SharedControl) {
    let mut ticker = embassy_time::Ticker::every(Duration::from_millis(500));
    loop {
        {
            let mut control = control.0.lock().await;
            let _ = control.gpio_set(0, true).await;
        }
        ticker.next().await;
        {
            let mut control = control.0.lock().await;
            let _ = control.gpio_set(0, false).await;
        }
        ticker.next().await;
    }
}

