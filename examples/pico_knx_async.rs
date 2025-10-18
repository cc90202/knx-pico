//! Practical example: KNX control with Pico 2 W
//!
//! This example demonstrates how to use AsyncTunnelClient to:
//! - Connect to a KNX gateway via WiFi
//! - Send GroupValue_Write commands (e.g., turn on/off lights)
//! - Receive GroupValue_Indication events from the bus
//!
//! ## Hardware Requirements
//! - Raspberry Pi Pico 2 W
//! - KNX gateway on local network (e.g., at 192.168.1.10:3671)
//!
//! ## Configuration
//! Modify the constants below before compiling:
//! - WIFI_SSID: Your WiFi network name
//! - WIFI_PASSWORD: Your WiFi password
//! - KNX_GATEWAY_IP: IP address of KNX gateway

#![no_std]
#![no_main]

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_net::udp::PacketMetadata;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0, USB};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use {defmt_rtt as _, panic_persist as _};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};

use knx_rs::addressing::GroupAddress;
use knx_rs::protocol::async_tunnel::AsyncTunnelClient;
use knx_rs::protocol::cemi::{ControlField1, ControlField2, Apci};
use knx_rs::protocol::constants::CEMIMessageCode;
use knx_rs::addressing::IndividualAddress;

// ============================================================================
// Configuration
// ============================================================================

const WIFI_SSID: &str = "YOUR_WIFI_SSID";
const WIFI_PASSWORD: &str = "YOUR_WIFI_PASSWORD";

// KNX Gateway configuration (Mac IP address running the simulator)
const KNX_GATEWAY_IP: [u8; 4] = [192, 168, 1, 23];
const KNX_GATEWAY_PORT: u16 = 3671;

// Example KNX group addresses (3-level: main/middle/sub)
// 1/2/3 = 0x0A03, calculated as: (1 << 11) | (2 << 8) | 3
const LIGHT_LIVING_ROOM_RAW: u16 = 0x0A03; // 1/2/3
#[allow(dead_code)] // Reserved for additional examples
const LIGHT_BEDROOM_RAW: u16 = 0x0A04;     // 1/2/4

// Our virtual KNX device address (area.line.device = 1.1.1)
// Calculated as: (1 << 12) | (1 << 8) | 1 = 0x1101
const DEVICE_ADDRESS_RAW: u16 = 0x1101; // 1.1.1

// ============================================================================
// Interrupt Bindings
// ============================================================================

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

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
// USB Logger Task
// ============================================================================

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

    // Start USB logger (must be first)
    let driver = Driver::new(p.USB, UsbIrqs);
    spawner.must_spawn(logger_task(driver));

    // Check for panic messages
    if let Some(panic_message) = panic_persist::get_panic_message_utf8() {
        log::error!("{panic_message}");
        loop {
            Timer::after(Duration::from_secs(5)).await;
        }
    }

    log::info!("Starting Pico 2 W KNX Example");

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

    log::info!("Connecting to WiFi: {}", WIFI_SSID);

    loop {
        match control
            .join(
                WIFI_SSID,
                cyw43::JoinOptions::new(WIFI_PASSWORD.as_bytes()),
            )
            .await
        {
            Ok(_) => {
                log::info!("WiFi connected successfully!");
                break;
            }
            Err(e) => {
                log::error!("WiFi connection failed: status={}, retrying in 5s...", e.status);
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
    log::info!("Waiting for IP address...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        log::info!("IP Address: {:?}", config.address.address());
    }

    // ========================================================================
    // KNX Connection
    // ========================================================================

    log::info!("Connecting to KNX gateway at {}.{}.{}.{}:{}",
          KNX_GATEWAY_IP[0], KNX_GATEWAY_IP[1], KNX_GATEWAY_IP[2], KNX_GATEWAY_IP[3], KNX_GATEWAY_PORT);

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
        KNX_GATEWAY_IP,
        KNX_GATEWAY_PORT,
    );

    // Connect to gateway
    match client.connect().await {
        Ok(_) => log::info!("✓ Connected to KNX gateway"),
        Err(_e) => {
            log::error!("Failed to connect to KNX gateway");
            return;
        }
    }

    // ========================================================================
    // Example 1: Turn on living room light
    // ========================================================================

    log::info!("Example 1: Turning ON living room light (1/2/3)");

    let light_addr = GroupAddress::from(LIGHT_LIVING_ROOM_RAW);
    let cemi_on = build_group_write_bool(light_addr, true);
    match client.send_cemi(&cemi_on).await {
        Ok(_) => log::info!("✓ Command sent successfully"),
        Err(_e) => log::error!("Failed to send command"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 2: Turn off living room light
    // ========================================================================

    log::info!("Example 2: Turning OFF living room light");

    let cemi_off = build_group_write_bool(light_addr, false);
    match client.send_cemi(&cemi_off).await {
        Ok(_) => log::info!("✓ Command sent successfully"),
        Err(_e) => log::error!("Failed to send command"),
    }

    Timer::after(Duration::from_secs(2)).await;

    // ========================================================================
    // Example 3: Listen for events from KNX bus
    // ========================================================================

    log::info!("Example 3: Listening for KNX bus events (press Ctrl+C to stop)...");

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

    loop {
        match client.receive().await {
            Ok(Some(cemi_data)) => {
                log::info!("Received cEMI frame ({} bytes)", cemi_data.len());

                // Parse the cEMI frame
                if let Ok(cemi) = knx_rs::protocol::cemi::CEMIFrame::parse(cemi_data) {
                    if let Ok(ldata) = cemi.as_ldata() {
                        if ldata.is_group_write() {
                            if let Some(dest) = ldata.destination_group() {
                                let dest_raw: u16 = dest.into();
                                log::info!("  GroupValue_Write to {:04X}: {} bytes",
                                      dest_raw, ldata.data.len());

                                // Example: decode boolean value (DPT 1)
                                if ldata.data.is_empty() {
                                    // Value encoded in APCI (6-bit)
                                    if let Apci::GroupValueWrite = ldata.apci {
                                        // For proper decoding, would need actual APCI byte
                                        log::info!("    Boolean value (encoded in APCI)");
                                    }
                                }
                            }
                        } else if ldata.is_group_read() {
                            if let Some(dest) = ldata.destination_group() {
                                let dest_raw: u16 = dest.into();
                                log::info!("  GroupValue_Read from {:04X}", dest_raw);
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                // No data available (timeout)
            }
            Err(_e) => {
                log::error!("Receive error");
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
