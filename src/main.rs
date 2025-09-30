#![no_std]
#![no_main]

mod configuration;
mod utility;
mod knx_client;

use crate::utility::*;
use crate::knx_client::{KnxClient, KnxEvent, KnxValue, format_group_address};
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
use embassy_net::udp::PacketMetadata;

// KNX imports
use knx_rs::addressing::GroupAddress;

// Conditional logging macros
#[cfg(feature = "usb-logger")]
macro_rules! info {
    ($($arg:tt)*) => {
        log::info!($($arg)*)
    };
}

#[cfg(feature = "usb-logger")]
macro_rules! error {
    ($($arg:tt)*) => {
        log::error!($($arg)*)
    };
}

#[cfg(not(feature = "usb-logger"))]
use defmt::{info, error};

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

/// Struttura per condividere il controller tra task embassy diversi
#[derive(Clone, Copy)]
pub struct SharedControl(&'static Mutex<CriticalSectionRawMutex, Control<'static>>);

/// Entry point principale secondo Embassy
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Parte il logger appropriato in base alla feature attiva
    #[cfg(feature = "usb-logger")]
    {
        let driver = Driver::new(p.USB, UsbIrqs);
        spawner.must_spawn(logger_task(driver));
    }

    if let Some(panic_message) = panic_persist::get_panic_message_utf8() {
        error!("{}", panic_message);
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

    // WiFi connection configuration from configuration.rs
    let wifi_ssid = get_ssid();
    let wifi_password = get_wifi_password();

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

    // ========================================================================
    // KNX Connection Test
    // ========================================================================

    // KNX Gateway configuration from configuration.rs
    let gateway_ip_str = get_knx_gateway_ip();
    let knx_gateway_ip = parse_ip(gateway_ip_str);
    let knx_gateway_port = 3671;

    info!("KNX Gateway configured: {}", gateway_ip_str);

    info!("Connecting to KNX gateway at {}.{}.{}.{}:{}",
          knx_gateway_ip[0], knx_gateway_ip[1], knx_gateway_ip[2], knx_gateway_ip[3], knx_gateway_port);

    // Allocate buffers for AsyncTunnelClient
    static RX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
    static TX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
    static RX_BUFFER: StaticCell<[u8; 2048]> = StaticCell::new();
    static TX_BUFFER: StaticCell<[u8; 2048]> = StaticCell::new();

    let rx_meta = RX_META.init([PacketMetadata::EMPTY; 4]);
    let tx_meta = TX_META.init([PacketMetadata::EMPTY; 4]);
    let rx_buffer = RX_BUFFER.init([0u8; 2048]);
    let tx_buffer = TX_BUFFER.init([0u8; 2048]);

    let mut client = KnxClient::new(
        &stack,
        rx_meta,
        tx_meta,
        rx_buffer,
        tx_buffer,
        knx_gateway_ip,
        knx_gateway_port,
    );

    // Connect to gateway
    info!("Attempting to connect...");
    match client.connect().await {
        Ok(_) => info!("✓ Connected to KNX gateway!"),
        Err(_) => {
            error!("✗ Failed to connect to KNX gateway");
            loop {
                Timer::after(Duration::from_secs(10)).await;
            }
        }
    }

    // Test: Send boolean commands to group address 1/2/3
    let light_addr = GroupAddress::from(0x0A03); // 1/2/3

    info!("Sending test: bool=true to 1/2/3");
    match client.write(light_addr, KnxValue::Bool(true)).await {
        Ok(_) => info!("✓ Command sent successfully"),
        Err(_) => error!("✗ Failed to send command"),
    }

    Timer::after(Duration::from_secs(2)).await;

    info!("Sending test: bool=false to 1/2/3");
    match client.write(light_addr, KnxValue::Bool(false)).await {
        Ok(_) => info!("✓ Command sent successfully"),
        Err(_) => error!("✗ Failed to send command"),
    }

    // Listen for KNX bus events
    info!("Listening for KNX bus events...");
    loop {
        match client.receive_event().await {
            Ok(Some(event)) => {
                match event {
                    KnxEvent::GroupWrite { address, value } => {
                        let (main, middle, sub) = format_group_address(address);
                        match value {
                            KnxValue::Bool(on) => {
                                info!(
                                    "💡 Switch {}/{}/{}: {}",
                                    main,
                                    middle,
                                    sub,
                                    if on { "ON" } else { "OFF" }
                                );
                            }
                            KnxValue::Percent(p) => {
                                info!(
                                    "📊 Dimmer {}/{}/{}: {}%",
                                    main,
                                    middle,
                                    sub,
                                    p
                                );
                            }
                            KnxValue::Temperature(t) => {
                                // Convert to fixed-point for display (1 decimal place)
                                let temp_int = (t * 10.0) as i32;
                                let whole = temp_int / 10;
                                let frac = (temp_int % 10).abs();
                                info!(
                                    "🌡️  Sensor {}/{}/{}: {}.{}°C",
                                    main,
                                    middle,
                                    sub,
                                    whole,
                                    frac
                                );
                            }
                        }
                    }
                    KnxEvent::GroupRead { address } => {
                        let (main, middle, sub) = format_group_address(address);
                        info!("📖 Value read request from {}/{}/{}", main, middle, sub);
                    }
                    KnxEvent::Unknown { address, data_len } => {
                        let (main, middle, sub) = format_group_address(address);
                        info!("❓ Unknown event at {}/{}/{} ({} bytes)", main, middle, sub, data_len);
                    }
                }
            }
            Ok(None) => {
                // No data (timeout)
            }
            Err(_) => {
                error!("❌ Receive error");
            }
        }

        Timer::after(Duration::from_millis(100)).await;
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

#[cfg(feature = "usb-logger")]
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

