//! Practical example: KNX control with Pico 2 W
//!
//! This example demonstrates how to use AsyncTunnelClient to:
//! - Connect to a KNX gateway via WiFi
//! - Send GroupValue_Write commands (e.g., turn on/off lights)
//! - Receive GroupValue_Indication events from the bus
//!
//! ## Hardware Requirements
//! - Raspberry Pi Pico 2 W
//! - KNX gateway on local network OR KNX simulator (see TESTING.md)
//!
//! ## Configuration
//! Edit `src/configuration.rs` to set your WiFi credentials:
//!    - `WIFI_NETWORK`: Your WiFi network name
//!    - `WIFI_PASSWORD`: Your WiFi password
//!
//! The KNX gateway is automatically discovered via multicast - no manual configuration needed!
//!
//! ## Flash to Pico
//!
//! **With USB logger (recommended):**
//! ```bash
//! cargo flash-example-usb
//! # Monitor: screen /dev/tty.usbmodem* 115200
//! ```
//!
//! **With defmt logger (requires probe):**
//! ```bash
//! # Build and flash
//! cargo build --release --example pico_knx_async --target thumbv8m.main-none-eabihf --features embassy-rp
//! probe-rs run --chip RP2350 target/thumbv8m.main-none-eabihf/release/examples/pico_knx_async
//! ```

#![no_std]
#![no_main]

mod common;

use common::knx_discovery;
use common::utility::{get_ssid, get_wifi_password};
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_net::udp::PacketMetadata;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use {defmt_rtt as _, panic_persist as _};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};

// Conditional imports for USB logger
#[cfg(feature = "usb-logger")]
use embassy_rp::peripherals::USB;
#[cfg(feature = "usb-logger")]
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};

use knx_rs::addressing::GroupAddress;
use knx_rs::protocol::async_tunnel::AsyncTunnelClient;
use knx_rs::protocol::cemi::{ControlField1, ControlField2, Apci};
use knx_rs::protocol::constants::CEMIMessageCode;
use knx_rs::addressing::IndividualAddress;
use knx_rs::{pico_log, ga};

// ============================================================================
// Configuration
// ============================================================================
// WiFi credentials are loaded from src/configuration.rs
// Edit that file to set your WIFI_NETWORK and WIFI_PASSWORD

// KNX Gateway Discovery
// The gateway is automatically discovered via multicast SEARCH_REQUEST
// No manual configuration needed!

// Example KNX group addresses
// Note: These are example addresses - adjust to match your KNX installation
// Using ga! macro for readable 3-level addressing (main/middle/sub)

// Our virtual KNX device address (area.line.device = 1.1.1)
// Calculated as: (1 << 12) | (1 << 8) | 1 = 0x1101
const DEVICE_ADDRESS_RAW: u16 = 0x1101; // 1.1.1

// ============================================================================
// Interrupt Bindings
// ============================================================================

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[cfg(feature = "usb-logger")]
bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

// ============================================================================
// Network Task
// ============================================================================

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

// ============================================================================
// WiFi Task
// ============================================================================

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

// ============================================================================
// USB Logger Task (only with usb-logger feature)
// ============================================================================

#[cfg(feature = "usb-logger")]
#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

// ============================================================================
// LED Blink Task (heartbeat indicator)
// ============================================================================

/// Shared control wrapper for cyw43 Control
#[derive(Clone, Copy)]
struct SharedControl(&'static Mutex<CriticalSectionRawMutex, cyw43::Control<'static>>);

#[embassy_executor::task]
async fn blink_task(shared_control: SharedControl) -> ! {
    loop {
        shared_control.0.lock().await.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(100)).await;
        shared_control.0.lock().await.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(900)).await;
    }
}


// ============================================================================
// Main Application
// ============================================================================

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Start appropriate logger based on active feature
    #[cfg(feature = "usb-logger")]
    {
        let driver = Driver::new(p.USB, UsbIrqs);
        spawner.must_spawn(logger_task(driver));
    }

    // Check for panic messages
    if let Some(panic_message) = panic_persist::get_panic_message_utf8() {
        pico_log!(error, "{}", panic_message);
        loop {
            Timer::after(Duration::from_secs(5)).await;
        }
    }

    pico_log!(info, "Starting Pico 2 W KNX Example");

    // ========================================================================
    // WiFi Initialization
    // ========================================================================

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
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // ========================================================================
    // Network Stack Setup
    // ========================================================================

    let config = Config::dhcpv4(Default::default());
    let seed = 0x1234_5678_u64;

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    // ========================================================================
    // WiFi Connection
    // ========================================================================

    // Load WiFi credentials from configuration.rs
    let wifi_ssid = get_ssid();
    let wifi_password = get_wifi_password();

    pico_log!(info, "Connecting to WiFi: {}", wifi_ssid);

    loop {
        match control
            .join(
                wifi_ssid,
                cyw43::JoinOptions::new(wifi_password.as_bytes()),
            )
            .await
        {
            Ok(_) => {
                pico_log!(info, "WiFi connected successfully!");
                break;
            }
            Err(e) => {
                pico_log!(error, "WiFi connection failed: status={}, retrying in 5s...", e.status);
                Timer::after(Duration::from_secs(5)).await;
            }
        }
    }

    // Create shared control for LED blink task (after WiFi connection)
    static CONTROL_MUTEX: StaticCell<Mutex<CriticalSectionRawMutex, cyw43::Control<'static>>> = StaticCell::new();
    let shared_control = SharedControl(CONTROL_MUTEX.init(Mutex::new(control)));

    // Start LED blink task
    unwrap!(spawner.spawn(blink_task(shared_control)));

    // Wait for DHCP
    pico_log!(info, "Waiting for IP address...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        pico_log!(info, "IP Address: {:?}", config.address.address());
    }

    // ========================================================================
    // KNX Gateway Discovery
    // ========================================================================

    pico_log!(info, "Discovering KNX gateway via multicast...");

    let (knx_gateway_ip, knx_gateway_port) = match knx_discovery::discover_gateway(&stack, Duration::from_secs(3)).await {
        Some(gateway) => {
            pico_log!(info, "✓ KNX Gateway discovered automatically!");
            pico_log!(info, "  IP: {}.{}.{}.{}", gateway.ip[0], gateway.ip[1], gateway.ip[2], gateway.ip[3]);
            pico_log!(info, "  Port: {}", gateway.port);
            (gateway.ip, gateway.port)
        }
        None => {
            pico_log!(error, "✗ No KNX gateway found on network!");
            pico_log!(error, "  Ensure your KNX gateway or simulator is running");
            pico_log!(error, "  and connected to the same network.");
            pico_log!(info, "System halted. Reset device to retry.");
            loop {
                Timer::after(Duration::from_secs(30)).await;
            }
        }
    };

    // ========================================================================
    // KNX Connection
    // ========================================================================

    pico_log!(info, "Connecting to KNX gateway at {}.{}.{}.{}:{}",
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

    let mut client = AsyncTunnelClient::new(
        &stack,
        rx_meta,
        tx_meta,
        rx_buffer,
        tx_buffer,
        knx_gateway_ip,
        knx_gateway_port,
    );

    // Connect to gateway
    match client.connect().await {
        Ok(_) => pico_log!(info, "✓ Connected to KNX gateway"),
        Err(_e) => {
            pico_log!(error, "Failed to connect to KNX gateway");
            return;
        }
    }

    // ========================================================================
    // Example 1: Turn on living room light
    // ========================================================================

    pico_log!(info, "Example 1: Turning ON living room light (1/2/3)");

    let light_addr = ga!(1/2/3);
    let cemi_on = build_group_write_bool(light_addr, true);
    match client.send_cemi(&cemi_on).await {
        Ok(_) => pico_log!(info, "✓ Command sent successfully"),
        Err(_e) => pico_log!(error, "Failed to send command"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 2: Turn off living room light
    // ========================================================================

    pico_log!(info, "Example 2: Turning OFF living room light");

    let cemi_off = build_group_write_bool(light_addr, false);
    match client.send_cemi(&cemi_off).await {
        Ok(_) => pico_log!(info, "✓ Command sent successfully"),
        Err(_e) => pico_log!(error, "Failed to send command"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 3: DPT 3 - Dimming Control (increase brightness)
    // ========================================================================

    pico_log!(info, "Example 3: DPT 3 - Dimmer increase brightness (4 steps)");

    let dimmer_addr = ga!(1/2/5);
    // DPT 3.007: Control Dimming
    // Byte format: cccc SUUU
    //   c = control (0011 = increase/decrease command)
    //   S = step flag (1 = increase, 0 = decrease)
    //   U = step code (number of intervals, 0-7)
    // Example: 0x0B = 0000 1011 = increase by 4 steps
    let cemi_dim_up = build_group_write_dpt3(dimmer_addr, 0x0B);
    match client.send_cemi(&cemi_dim_up).await {
        Ok(_) => pico_log!(info, "✓ Dimmer command sent"),
        Err(_e) => pico_log!(error, "Failed to send dimmer command"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 4: DPT 5 - Percentage (0-100%)
    // ========================================================================

    pico_log!(info, "Example 4: DPT 5 - Set valve position to 75%");

    let valve_addr = ga!(1/2/6);
    // DPT 5.001: Percentage (0-100%)
    // Range: 0x00 (0%) to 0xFF (100%)
    // 75% = 0xFF * 0.75 = 191 = 0xBF
    let cemi_valve = build_group_write_dpt5(valve_addr, 0xBF);
    match client.send_cemi(&cemi_valve).await {
        Ok(_) => pico_log!(info, "✓ Valve position set to 75%"),
        Err(_e) => pico_log!(error, "Failed to set valve position"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 5: DPT 9 - Temperature (write 21.5°C)
    // ========================================================================

    pico_log!(info, "Example 5: DPT 9 - Set temperature setpoint to 21.5°C");

    let temp_addr = ga!(1/2/7);  // Temperature sensor/setpoint
    // DPT 9.001: Temperature (2-byte float)
    // Format: MEEE EMMM MMMM MMMM
    //   M = mantissa (11-bit signed)
    //   E = exponent (4-bit signed)
    // Value = (0.01 * M) * 2^E
    // For 21.5°C: M=2150, E=0
    // Encoding: 0x0C 0x66 (calculated for 21.5)
    let cemi_temp = build_group_write_dpt9(temp_addr, 0x0C, 0x66);
    match client.send_cemi(&cemi_temp).await {
        Ok(_) => pico_log!(info, "✓ Temperature setpoint written"),
        Err(_e) => pico_log!(error, "Failed to write temperature"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 6: Listen for events from KNX bus
    // ========================================================================

    pico_log!(info, "Example 6: Listening for KNX bus events (press Ctrl+C to stop)...");
    pico_log!(info, "");
    pico_log!(info, "=== All examples completed successfully! ===");
    pico_log!(info, "Now entering passive monitoring mode...");
    pico_log!(info, "The device will display any KNX traffic on the bus.");
    pico_log!(info, "");

    // NOTE: In a real application, you should call client.send_heartbeat()
    // every 60 seconds to keep the connection alive. The gateway will close
    // the connection if no heartbeat is received.
    //
    // Example heartbeat implementation:
    // let mut last_heartbeat = embassy_time::Instant::now();
    // if last_heartbeat.elapsed() > Duration::from_secs(60) {
    //     client.send_heartbeat().await?;
    //     last_heartbeat = embassy_time::Instant::now();
    // }

    let mut event_count = 0;
    loop {
        match client.receive().await {
            Ok(Some(cemi_data)) => {
                event_count += 1;
                pico_log!(info, "[Event #{}] Received cEMI frame ({} bytes)", event_count, cemi_data.len());

                // Parse the cEMI frame
                if let Ok(cemi) = knx_rs::protocol::cemi::CEMIFrame::parse(cemi_data) {
                    if let Ok(ldata) = cemi.as_ldata() {
                        if ldata.is_group_write() {
                            if let Some(dest) = ldata.destination_group() {
                                let dest_raw: u16 = dest.into();
                                pico_log!(info, "  GroupValue_Write to {:04X}: {} bytes",
                                      dest_raw, ldata.data.len());

                                // Example: decode boolean value (DPT 1)
                                if ldata.data.is_empty() {
                                    // Value encoded in APCI (6-bit)
                                    if let Apci::GroupValueWrite = ldata.apci {
                                        // For proper decoding, would need actual APCI byte
                                        pico_log!(info, "    Boolean value (encoded in APCI)");
                                    }
                                }
                            }
                        } else if ldata.is_group_read() {
                            if let Some(dest) = ldata.destination_group() {
                                let dest_raw: u16 = dest.into();
                                pico_log!(info, "  GroupValue_Read from {:04X}", dest_raw);
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                // No data available (timeout)
            }
            Err(_e) => {
                pico_log!(error, "Receive error");
            }
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build a cEMI L_Data.req frame for GroupValue_Write with boolean value
///
/// This constructs a complete cEMI frame for writing a DPT 1 (boolean) value
/// to a group address.
fn build_group_write_bool(group_addr: GroupAddress, value: bool) -> [u8; 11] {
    let mut frame = [0u8; 11];

    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    // Message code: L_Data.req
    frame[0] = CEMIMessageCode::LDataReq.to_u8();

    // Additional info length: 0 (no additional info)
    frame[1] = 0x00;

    // Control field 1: Standard frame, repeat allowed, broadcast, normal priority
    frame[2] = ControlField1::default().raw();

    // Control field 2: Group address, hop count 6
    frame[3] = ControlField2::default().raw();

    // Source address: Our device address (1.1.1)
    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    // Destination address: Group address
    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    // NPDU length: 1 byte (TPCI/APCI with 6-bit value)
    frame[8] = 0x01;

    // TPCI (bits 7-6: 00 = UnnumberedData) + APCI bits 9-8 (bits 1-0: 00 for GroupValueWrite)
    frame[9] = 0x00;

    // APCI bits 7-6 (10 = GroupValueWrite) + 6-bit data value
    // GroupValueWrite = 0x080, so bits 7-6 = 10
    // For boolean: 0 or 1 in lowest bit
    let apci_data = if value { 0x81 } else { 0x80 };
    frame[10] = apci_data;

    frame
}

/// Build a cEMI L_Data.req frame for GroupValue_Read
#[allow(dead_code)]
fn build_group_read(group_addr: GroupAddress) -> [u8; 10] {
    let mut frame = [0u8; 10];

    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    frame[0] = CEMIMessageCode::LDataReq.to_u8();
    frame[1] = 0x00;
    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    // NPDU length: 1 byte (TPCI/APCI only)
    frame[8] = 0x01;

    // TPCI + APCI
    frame[9] = 0x00; // TPCI (unnumbered data) + APCI (GroupValueRead = 0x000)

    frame
}

/// Build a cEMI L_Data.req frame for GroupValue_Write with DPT 3 (4-bit control)
///
/// DPT 3.007: Dimming Control
/// DPT 3.008: Blinds Control
///
/// Byte format: cccc SUUU
///   - c = control field (always 0000 for step commands)
///   - S = step direction (1 = increase/up, 0 = decrease/down)
///   - U = step code (0-7, number of intervals)
///
/// Common values:
/// - 0x01: Decrease by 1 step
/// - 0x09: Increase by 1 step (0000 1001)
/// - 0x0B: Increase by 4 steps (0000 1011)
/// - 0x0F: Increase by 100% (0000 1111)
fn build_group_write_dpt3(group_addr: GroupAddress, value: u8) -> [u8; 11] {
    let mut frame = [0u8; 11];

    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    frame[0] = CEMIMessageCode::LDataReq.to_u8();
    frame[1] = 0x00;
    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    // NPDU length: 1 byte (TPCI/APCI with 6-bit value)
    frame[8] = 0x01;

    // TPCI + APCI high bits
    frame[9] = 0x00;

    // APCI GroupValueWrite (0x80) + 4-bit value in lower nibble
    frame[10] = 0x80 | (value & 0x0F);

    frame
}

/// Build a cEMI L_Data.req frame for GroupValue_Write with DPT 5 (8-bit unsigned)
///
/// DPT 5.001: Percentage (0-100%)
/// DPT 5.003: Angle (0-360 degrees)
/// DPT 5.010: Counter (0-255)
///
/// Value range: 0x00 (0%) to 0xFF (100%)
fn build_group_write_dpt5(group_addr: GroupAddress, value: u8) -> [u8; 12] {
    let mut frame = [0u8; 12];

    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    frame[0] = CEMIMessageCode::LDataReq.to_u8();
    frame[1] = 0x00;
    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    // NPDU length: 2 bytes (TPCI/APCI + 1 data byte)
    frame[8] = 0x02;

    // TPCI + APCI high bits
    frame[9] = 0x00;

    // APCI GroupValueWrite (0x80)
    frame[10] = 0x80;

    // Data value
    frame[11] = value;

    frame
}

/// Build a cEMI L_Data.req frame for GroupValue_Write with DPT 9 (2-byte float)
///
/// DPT 9.001: Temperature (°C)
/// DPT 9.004: Illuminance (lux)
/// DPT 9.005: Wind speed (m/s)
///
/// Format: MEEE EMMM MMMM MMMM
///   - M = mantissa (11-bit signed, -2048 to 2047)
///   - E = exponent (4-bit signed, -8 to 7)
///
/// Value = (0.01 * M) * 2^E
///
/// Examples:
///   - 21.5°C: high=0x0C, low=0x66 (M=2150, E=0)
///   - -5.0°C: high=0x87, low=0x0C (M=-500, E=0)
///   - 100.0: high=0x2E, low=0x10 (M=625, E=4)
fn build_group_write_dpt9(group_addr: GroupAddress, high: u8, low: u8) -> [u8; 13] {
    let mut frame = [0u8; 13];

    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    frame[0] = CEMIMessageCode::LDataReq.to_u8();
    frame[1] = 0x00;
    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    // NPDU length: 3 bytes (TPCI/APCI + 2 data bytes)
    frame[8] = 0x03;

    // TPCI + APCI high bits
    frame[9] = 0x00;

    // APCI GroupValueWrite (0x80)
    frame[10] = 0x80;

    // Data bytes (2-byte float)
    frame[11] = high;
    frame[12] = low;

    frame
}
