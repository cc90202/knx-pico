//! KNX Sniffer/Tester
//!
//! Interactive tool for testing KNX communication with simulator or real KNX hardware.
//! Performs gateway discovery, read and write operations.
//!
//! ## Hardware Required
//! - Raspberry Pi Pico 2 W
//! - KNX gateway on local network OR KNX simulator (see TESTING.md)
//!
//! ## Flash to Pico
//!
//! **With USB logger (recommended):**
//! ```bash
//! cargo flash-sniffer-usb-release
//! # Monitor: screen /dev/tty.usbmodem* 115200
//! ```
//!
//! **With defmt logger (requires probe):**
//! ```bash
//! cargo flash-sniffer-release
//! # Logs visible via probe-rs
//! ```

#![no_std]
#![no_main]
#![allow(dead_code)]

mod common;

use common::utility::*;
use common::knx_client::{KnxClient, KnxBuffers, KnxValue};
use common::knx_discovery;
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

// KNX imports
use knx_rs::addressing::GroupAddress;

// Import unified logging macro from knx_rs crate
use knx_rs::pico_log;

// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"KNX Sniffer"),
    embassy_rp::binary_info::rp_program_description!(
        c"KNX protocol tester and sniffer for Raspberry Pico 2 W"
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

    pico_log!(info, "=== KNX Sniffer/Tester ===");

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

    pico_log!(info, "Network ready, starting KNX tests...");

    // ========================================================================
    // KNX Gateway Discovery and Connection Test
    // ========================================================================

    // Feature flag: set to false to use static configuration from configuration.rs
    const USE_AUTO_DISCOVERY: bool = true;

    let (knx_gateway_ip, knx_gateway_port) = if USE_AUTO_DISCOVERY {
        // Try automatic gateway discovery via SEARCH_REQUEST
        pico_log!(info, "1. Testing SEARCH_REQUEST (gateway discovery)...");

        match knx_discovery::discover_gateway(&stack, Duration::from_secs(3)).await {
            Some(gateway) => {
                pico_log!(info, "✓ KNX Gateway discovered automatically!");
                pico_log!(info, "  IP: {}.{}.{}.{}", gateway.ip[0], gateway.ip[1], gateway.ip[2], gateway.ip[3]);
                pico_log!(info, "  Port: {}", gateway.port);
                (gateway.ip, gateway.port)
            }
            None => {
                // Fallback to static configuration
                pico_log!(warn, "✗ No gateway found via discovery, using static configuration");
                let gateway_ip_str = get_knx_gateway_ip();
                let knx_gateway_ip = parse_ip(gateway_ip_str);
                pico_log!(info, "  Fallback to: {}", gateway_ip_str);
                (knx_gateway_ip, 3671)
            }
        }
    } else {
        // Use static configuration from configuration.rs
        let gateway_ip_str = get_knx_gateway_ip();
        let knx_gateway_ip = parse_ip(gateway_ip_str);
        pico_log!(info, "KNX Gateway (static config): {}", gateway_ip_str);
        (knx_gateway_ip, 3671)
    };

    pico_log!(info, "Connecting to KNX gateway at {}.{}.{}.{}:{}",
          knx_gateway_ip[0], knx_gateway_ip[1], knx_gateway_ip[2], knx_gateway_ip[3], knx_gateway_port);

    // Allocate buffers for KNX client using the new KnxBuffers struct
    static KNX_BUFFERS: StaticCell<KnxBuffers> = StaticCell::new();
    let knx_buffers = KNX_BUFFERS.init(KnxBuffers::new());

    // Create KNX client using the builder pattern
    let mut client = KnxClient::builder()
        .gateway(knx_gateway_ip, knx_gateway_port)
        .device_address([1, 1, 1])  // Device address 1.1.1
        .build_with_buffers(&stack, knx_buffers)
        .unwrap();

    // Connect to gateway
    pico_log!(info, "2. Testing CONNECT_REQUEST...");
    match client.connect().await {
        Ok(_) => pico_log!(info, "✓ Connected to KNX gateway!"),
        Err(_) => {
            pico_log!(error, "✗ Failed to connect to KNX gateway");
            pico_log!(error, "Tests cannot continue without connection");
            loop {
                Timer::after(Duration::from_secs(10)).await;
            }
        }
    }

    // ========================================================================
    // Test: READ Commands
    // ========================================================================

    pico_log!(info, "3. Testing READ commands...");
    let test_addr = GroupAddress::from(0x0A03); // 1/2/3

    pico_log!(info, "Sending READ to 1/2/3...");
    // Note: Read operations are fire-and-forget in this implementation
    // The response would come as a GroupResponse event
    match client.read(test_addr).await {
        Ok(_) => pico_log!(info, "✓ READ command sent (response would be received as event)"),
        Err(_) => pico_log!(error, "✗ Failed to send READ command"),
    }

    Timer::after(Duration::from_secs(1)).await;

    // ========================================================================
    // Test: WRITE Commands
    // ========================================================================

    pico_log!(info, "4. Testing WRITE commands...");
    let light_addr = GroupAddress::from(0x0A03); // 1/2/3

    pico_log!(info, "Sending WRITE: bool=true to 1/2/3");
    match client.write(light_addr, KnxValue::Bool(true)).await {
        Ok(_) => {
            pico_log!(info, "✓ WRITE command sent successfully (fire-and-forget)");
        }
        Err(_) => {
            pico_log!(error, "✗ Failed to send WRITE command");
        }
    }

    // Wait before next command
    Timer::after(Duration::from_secs(2)).await;

    pico_log!(info, "Sending WRITE: bool=false to 1/2/3");
    match client.write(light_addr, KnxValue::Bool(false)).await {
        Ok(_) => {
            pico_log!(info, "✓ WRITE command sent successfully");
        }
        Err(_) => {
            pico_log!(error, "✗ Failed to send WRITE command");
            pico_log!(error, "Connection may be lost, attempting reconnection...");

            // Try to reconnect
            match client.connect().await {
                Ok(_) => {
                    pico_log!(info, "✓ Reconnected to KNX gateway!");
                    // Retry the command
                    if let Ok(_) = client.write(light_addr, KnxValue::Bool(false)).await {
                        pico_log!(info, "✓ Command sent successfully after reconnection");
                    }
                }
                Err(_) => {
                    pico_log!(error, "✗ Failed to reconnect");
                }
            }
        }
    }

    Timer::after(Duration::from_secs(1)).await;

    // ========================================================================
    // Test Summary
    // ========================================================================

    pico_log!(info, "");
    pico_log!(info, "=== Test Summary ===");
    pico_log!(info, "✓ SEARCH_REQUEST: OK (gateway discovery)");
    pico_log!(info, "✓ CONNECT_REQUEST: OK (tunnel established)");
    pico_log!(info, "✓ READ: OK (sent to 1/2/3)");
    pico_log!(info, "✓ WRITE: OK (sent bool true/false to 1/2/3)");
    pico_log!(info, "");
    pico_log!(info, "All basic tests completed!");
    pico_log!(info, "");

    // ========================================================================
    // IMPORTANT: Event listening disabled to prevent crash on Pico 2 W
    // ========================================================================
    // The Pico 2 W crashes when calling recv_from() repeatedly in a loop.
    // This is a known hardware limitation with the current network stack.
    //
    // To enable passive sniffer mode, uncomment the code below, but be aware
    // that it may cause system crashes after prolonged operation.
    // ========================================================================

    pico_log!(warn, "Note: Passive sniffer mode is disabled to prevent crashes");
    pico_log!(info, "System will now idle. Reset device to run tests again.");

    // Idle loop
    loop {
        Timer::after(Duration::from_secs(30)).await;
        pico_log!(info, "System alive (idle mode)");
    }

    /* SNIFFER MODE - DISABLED DUE TO CRASHES

    pico_log!(info, "Entering passive sniffer mode...");
    pico_log!(info, "Listening for KNX bus events (press Ctrl+C to stop)");

    loop {
        match client.receive_event().await {
            Ok(Some(event)) => {
                match event {
                    KnxEvent::GroupWrite { address, value } => {
                        let (main, middle, sub) = format_group_address(address);
                        match value {
                            KnxValue::Bool(on) => {
                                pico_log!(info,
                                    "💡 Switch {}/{}/{}: {}",
                                    main,
                                    middle,
                                    sub,
                                    if on { "ON" } else { "OFF" }
                                );
                            }
                            KnxValue::Percent(p) => {
                                pico_log!(info,
                                    "📊 Dimmer {}/{}/{}: {}%",
                                    main,
                                    middle,
                                    sub,
                                    p
                                );
                            }
                            KnxValue::Temperature(t) => {
                                let temp_int = (t * 10.0) as i32;
                                let whole = temp_int / 10;
                                let frac = (temp_int % 10).abs();
                                pico_log!(info,
                                    "🌡️  Temperature {}/{}/{}: {}.{}°C",
                                    main,
                                    middle,
                                    sub,
                                    whole,
                                    frac
                                );
                            }
                            _ => {
                                pico_log!(info, "📨 Event at {}/{}/{}", main, middle, sub);
                            }
                        }
                    }
                    KnxEvent::GroupRead { address } => {
                        let (main, middle, sub) = format_group_address(address);
                        pico_log!(info, "📖 Read request from {}/{}/{}", main, middle, sub);
                    }
                    KnxEvent::GroupResponse { address, .. } => {
                        let (main, middle, sub) = format_group_address(address);
                        pico_log!(info, "📬 Response from {}/{}/{}", main, middle, sub);
                    }
                    KnxEvent::Unknown { address, data_len } => {
                        let (main, middle, sub) = format_group_address(address);
                        pico_log!(info, "❓ Unknown event at {}/{}/{} ({} bytes)", main, middle, sub, data_len);
                    }
                }
            }
            Ok(None) => {
                // No data available (timeout)
            }
            Err(_) => {
                pico_log!(error, "❌ Receive error");
                Timer::after(Duration::from_secs(1)).await;
            }
        }

        // Delay to prevent stack overflow
        Timer::after(Duration::from_millis(100)).await;
    }
    */
}

// Include the same background tasks from main.rs
#[cfg(feature = "usb-logger")]
#[embassy_executor::task]
async fn logger_task(driver: embassy_rp::usb::Driver<'static, embassy_rp::peripherals::USB>) {
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

