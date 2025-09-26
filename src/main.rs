#![no_std]
#![no_main]

use cyw43::Control;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use defmt::unwrap;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0, USB};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use panic_persist as _;
use static_cell::StaticCell;

// Network stack imports
use embassy_net::{Config, StackResources};

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

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

/// Struttura per condividere il controller tra task embassy diversi
#[derive(Clone, Copy)]
pub struct SharedControl(&'static Mutex<CriticalSectionRawMutex, Control<'static>>);

/// Entry point principale secondo Embassy
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Parte il logger su USB
    let driver = Driver::new(p.USB, UsbIrqs);
    spawner.must_spawn(logger_task(driver));

    if let Some(panic_message) = panic_persist::get_panic_message_utf8() {
        log::error!("{panic_message}");
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
    info!("Random seed: {}", seed);

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

    // WiFi connection configuration
    // Set these via environment variables at build time:
    // WIFI_SSID=YourNetwork WIFI_PASSWORD=YourPassword cargo build-rp2040
    let wifi_ssid = option_env!("WIFI_SSID").unwrap_or("YOUR_WIFI_SSID");
    let wifi_password = option_env!("WIFI_PASSWORD").unwrap_or("YOUR_WIFI_PASSWORD");

    info!("Connecting to WiFi network: {}", wifi_ssid);

    // Join WiFi network
    loop {
        {
            let mut control = shared_control.0.lock().await;
            match control.join(wifi_ssid, cyw43::JoinOptions::new(wifi_password.as_bytes())).await {
                Ok(_) => {
                    info!("WiFi connected successfully!");
                    break;
                }
                Err(e) => {
                    error!("WiFi connection failed: status={}, retrying in 5s...", e.status);
                }
            }
        }
        Timer::after(Duration::from_secs(5)).await;
    }

    // Wait for DHCP to assign IP address
    info!("Waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }

    if let Some(config) = stack.config_v4() {
        info!("IP Address: {}", config.address);
        info!("Gateway: {:?}", config.gateway);
    }

    info!("KNX-RS initialized and network ready!");

    // TODO: Start KNX client here in Phase 4

    // Main loop
    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn blink_task(shared_control: SharedControl) {
    let delay = Duration::from_millis(500);
    loop {
        shared_control.0.lock().await.gpio_set(0, true).await;
        Timer::after(delay).await;
        shared_control.0.lock().await.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}
