//! Presence Detection Automation Example
//!
//! This example demonstrates intelligent lighting automation based on
//! motion detection, ambient light levels, and time of day. It's a
//! real-world scenario commonly used in smart homes and offices.
//!
//! # Automation Logic
//!
//! The system automatically controls lights based on:
//! - Motion detection (PIR sensors)
//! - Ambient brightness (lux sensors)
//! - Time of day (day/night modes)
//! - Timeout periods (auto-off after no motion)
//!
//! # Scenarios
//!
//! 1. **Hallway Automation**
//!    - Turn on lights when motion detected
//!    - Only if ambient brightness < 100 lux
//!    - Auto-off after 2 minutes of no motion
//!    - Dimmed to 30% during night (22:00-06:00)
//!
//! 2. **Bathroom Automation**
//!    - Turn on full brightness when motion detected
//!    - Always activate (ignore brightness sensor)
//!    - Auto-off after 5 minutes of no motion
//!    - Reduce to 20% after first minute (occupied mode)
//!
//! 3. **Office Automation**
//!    - Turn on lights when motion detected
//!    - Only if ambient brightness < 300 lux (office has windows)
//!    - Auto-off after 10 minutes of no motion
//!    - Maintain brightness while occupied
//!
//! 4. **Staircase Automation**
//!    - Turn on immediately when motion detected
//!    - Ignore brightness (safety requirement)
//!    - Auto-off after 1 minute
//!    - Full brightness always (safety)
//!
//! # KNX Architecture
//!
//! ```
//! Hallway:
//!   Motion sensor:   5/0/1 (DPT 1.001 - Binary)
//!   Brightness:      5/1/1 (DPT 7.013 - Lux)
//!   Light dimmer:    5/2/1 (DPT 5.001 - Percentage)
//!
//! Bathroom:
//!   Motion sensor:   6/0/1 (DPT 1.001 - Binary)
//!   Light switch:    6/2/1 (DPT 1.001 - Switch)
//!   Light dimmer:    6/2/2 (DPT 5.001 - Percentage)
//!
//! Office:
//!   Motion sensor:   7/0/1 (DPT 1.001 - Binary)
//!   Brightness:      7/1/1 (DPT 7.013 - Lux)
//!   Light switch:    7/2/1 (DPT 1.001 - Switch)
//!
//! Staircase:
//!   Motion sensor:   8/0/1 (DPT 1.001 - Binary)
//!   Light switch:    8/2/1 (DPT 1.001 - Switch)
//! ```
//!
//! # Setup
//!
//! 1. Start the simulator: `python3 knx_simulator.py --verbose`
//! 2. Run this example: `cargo run --example presence_detection_automation`
//!
//! Or connect to a real KNX gateway by changing the gateway IP below.

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};
use std::str::FromStr;
use std::collections::HashMap;

use knx_rs::protocol::tunnel::TunnelClient;
use knx_rs::addressing::GroupAddress;
use knx_rs::dpt::{Dpt1, Dpt5, DptEncode};

// Automation configuration constants
const HALLWAY_TIMEOUT_SECS: u64 = 120;      // 2 minutes
const BATHROOM_TIMEOUT_SECS: u64 = 300;     // 5 minutes
const BATHROOM_DIM_DELAY_SECS: u64 = 60;    // 1 minute before dimming
const OFFICE_TIMEOUT_SECS: u64 = 600;       // 10 minutes
const STAIRCASE_TIMEOUT_SECS: u64 = 60;     // 1 minute

const HALLWAY_LUX_THRESHOLD: u16 = 100;
const OFFICE_LUX_THRESHOLD: u16 = 300;

// Night mode configuration (not used in this demo, but available for future enhancement)
#[allow(dead_code)]
const NIGHT_START_HOUR: u8 = 22;  // 22:00
#[allow(dead_code)]
const NIGHT_END_HOUR: u8 = 6;     // 06:00

const UPDATE_INTERVAL_SECS: u64 = 5;  // Check sensors every 5 seconds

/// Represents the state of a single automation zone
#[derive(Debug)]
struct ZoneState {
    name: String,
    motion_detected: bool,
    last_motion_time: Option<Instant>,
    ambient_lux: Option<u16>,
    light_on: bool,
    light_level: u8,  // 0-100%
    timeout_secs: u64,
    lux_threshold: Option<u16>,  // None = always activate
    dimmed_mode: bool,  // For bathroom after first minute
    activation_count: u32,
}

impl ZoneState {
    fn new(name: &str, timeout_secs: u64, lux_threshold: Option<u16>) -> Self {
        Self {
            name: name.to_string(),
            motion_detected: false,
            last_motion_time: None,
            ambient_lux: None,
            light_on: false,
            light_level: 0,
            timeout_secs,
            lux_threshold,
            dimmed_mode: false,
            activation_count: 0,
        }
    }

    fn update_motion(&mut self, detected: bool) {
        self.motion_detected = detected;
        if detected {
            self.last_motion_time = Some(Instant::now());
        }
    }

    fn update_brightness(&mut self, lux: u16) {
        self.ambient_lux = Some(lux);
    }

    fn seconds_since_motion(&self) -> Option<u64> {
        self.last_motion_time.map(|t| t.elapsed().as_secs())
    }

    fn should_activate(&self) -> bool {
        if !self.motion_detected {
            return false;
        }

        // Check lux threshold if configured
        if let Some(threshold) = self.lux_threshold {
            if let Some(lux) = self.ambient_lux {
                if lux >= threshold {
                    return false;  // Too bright, don't activate
                }
            } else {
                // No lux reading, assume it's dark (safe default)
            }
        }

        true
    }

    fn should_deactivate(&self) -> bool {
        if !self.light_on {
            return false;
        }

        if let Some(elapsed) = self.seconds_since_motion() {
            elapsed >= self.timeout_secs
        } else {
            false
        }
    }

    fn activate(&mut self) {
        self.light_on = true;
        self.activation_count += 1;
        self.dimmed_mode = false;
    }

    fn deactivate(&mut self) {
        self.light_on = false;
        self.light_level = 0;
        self.dimmed_mode = false;
    }

    fn status_emoji(&self) -> &str {
        if self.light_on {
            if self.dimmed_mode {
                "🟡"
            } else {
                "🟢"
            }
        } else {
            "⚫"
        }
    }

    fn motion_emoji(&self) -> &str {
        if self.motion_detected { "🔴" } else { "⚪" }
    }
}

/// Check if current time is during night hours
fn is_night_time() -> bool {
    // This is a simplified version - in production you'd use chrono crate
    // For demo purposes, we'll just return false (always day mode)
    // In real implementation: get local time and check against NIGHT_START_HOUR/NIGHT_END_HOUR

    // Simplified: assume it's always day time for demo
    // In production:
    //   use chrono::Local;
    //   let hour = Local::now().hour();
    //   hour >= NIGHT_START_HOUR || hour < NIGHT_END_HOUR

    false  // Always day mode for demo
}

/// Read a boolean sensor (DPT 1.001)
fn read_motion(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    buffer: &mut [u8; 1024],
) -> Option<bool> {
    let mut cemi = Vec::new();
    cemi.push(0x11);
    cemi.push(0x00);
    cemi.push(0xBC);
    cemi.push(0xE0);
    cemi.push(0x11);
    cemi.push(0xFA);

    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);
    cemi.push(0x01);
    cemi.push(0x00);
    cemi.push(0x00);

    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        let _ = socket.send_to(frame, gateway_addr);

        if let Ok((len, _)) = socket.recv_from(buffer) {
            let _ = client.handle_tunneling_ack(&buffer[6..len]);

            socket.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
            if let Ok((len, _)) = socket.recv_from(buffer) {
                socket.set_read_timeout(Some(Duration::from_secs(2))).ok();

                if len > 10 && buffer[6] == 0x06 && buffer[7] == 0x10 {
                    let cemi_start = 10;
                    if buffer[cemi_start] == 0x29 {
                        let data_start = cemi_start + 10;
                        if len > data_start {
                            let apci_byte = buffer[data_start];
                            return Some((apci_byte & 0x01) != 0);
                        }
                    }
                }
            }
            socket.set_read_timeout(Some(Duration::from_secs(2))).ok();
        }
    }

    None
}

/// Read a 16-bit unsigned value (DPT 7.xxx for lux)
fn read_brightness(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    buffer: &mut [u8; 1024],
) -> Option<u16> {
    let mut cemi = Vec::new();
    cemi.push(0x11);
    cemi.push(0x00);
    cemi.push(0xBC);
    cemi.push(0xE0);
    cemi.push(0x11);
    cemi.push(0xFA);

    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);
    cemi.push(0x01);
    cemi.push(0x00);
    cemi.push(0x00);

    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        let _ = socket.send_to(frame, gateway_addr);

        if let Ok((len, _)) = socket.recv_from(buffer) {
            let _ = client.handle_tunneling_ack(&buffer[6..len]);

            socket.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
            if let Ok((len, _)) = socket.recv_from(buffer) {
                socket.set_read_timeout(Some(Duration::from_secs(2))).ok();

                if len > 10 && buffer[6] == 0x06 && buffer[7] == 0x10 {
                    let cemi_start = 10;
                    if buffer[cemi_start] == 0x29 {
                        let data_start = cemi_start + 10;
                        if len > data_start + 2 {
                            let value_bytes = &buffer[data_start+1..data_start+3];
                            let value = ((value_bytes[0] as u16) << 8) | (value_bytes[1] as u16);
                            return Some(value);
                        }
                    }
                }
            }
            socket.set_read_timeout(Some(Duration::from_secs(2))).ok();
        }
    }

    None
}

/// Write a switch value (DPT 1.001)
fn write_switch(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    value: bool,
    buffer: &mut [u8; 1024],
) -> bool {
    let mut cemi = Vec::new();
    cemi.push(0x11);
    cemi.push(0x00);
    cemi.push(0xBC);
    cemi.push(0xE0);
    cemi.push(0x11);
    cemi.push(0xFA);

    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);
    cemi.push(0x01);
    cemi.push(0x00);

    let data = Dpt1::Switch.encode(value).unwrap();
    cemi.push(0x80 | data[0]);

    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        if socket.send_to(frame, gateway_addr).is_ok() {
            if let Ok((len, _)) = socket.recv_from(buffer) {
                return client.handle_tunneling_ack(&buffer[6..len]).is_ok();
            }
        }
    }

    false
}

/// Write a dimmer percentage (DPT 5.001)
fn write_dimmer(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    percentage: u8,
    buffer: &mut [u8; 1024],
) -> bool {
    let mut cemi = Vec::new();
    cemi.push(0x11);
    cemi.push(0x00);
    cemi.push(0xBC);
    cemi.push(0xE0);
    cemi.push(0x11);
    cemi.push(0xFA);

    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);
    cemi.push(0x02);
    cemi.push(0x00);
    cemi.push(0x80);

    let byte = Dpt5::Percentage.encode_to_byte(percentage as u16).unwrap();
    cemi.push(byte);

    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        if socket.send_to(frame, gateway_addr).is_ok() {
            if let Ok((len, _)) = socket.recv_from(buffer) {
                return client.handle_tunneling_ack(&buffer[6..len]).is_ok();
            }
        }
    }

    false
}

/// Print the automation status dashboard
fn print_dashboard(zones: &HashMap<&str, ZoneState>, cycle_count: u32) {
    print!("\x1B[2J\x1B[1;1H");  // Clear screen

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║         🏠  PRESENCE DETECTION AUTOMATION  🏠                     ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    let zone_order = ["hallway", "bathroom", "office", "staircase"];

    for zone_key in &zone_order {
        if let Some(zone) = zones.get(zone_key) {
            println!("┌─────────────────────────────────────────────────────────────────┐");
            println!("│ {} {:<60} │", zone.status_emoji(), zone.name.to_uppercase());
            println!("├─────────────────────────────────────────────────────────────────┤");

            println!("│  Motion:       {} {}                                        │",
                     zone.motion_emoji(),
                     if zone.motion_detected { "Detected" } else { "No motion" });

            if let Some(lux) = zone.ambient_lux {
                println!("│  Brightness:   {:5} lux                                          │", lux);
            } else {
                println!("│  Brightness:   N/A                                               │");
            }

            println!("│  Light:        {}                                              │",
                     if zone.light_on {
                         if zone.light_level > 0 {
                             format!("ON ({}%)", zone.light_level)
                         } else {
                             "ON (100%)".to_string()
                         }
                     } else {
                         "OFF".to_string()
                     });

            if let Some(elapsed) = zone.seconds_since_motion() {
                let remaining = if zone.timeout_secs > elapsed {
                    zone.timeout_secs - elapsed
                } else {
                    0
                };
                println!("│  Auto-off in:  {}s (timeout: {}s)                         │",
                         remaining, zone.timeout_secs);
            } else {
                println!("│  Auto-off in:  -- (no motion detected yet)                       │");
            }

            println!("│  Activations:  {} times                                          │",
                     zone.activation_count);

            println!("└─────────────────────────────────────────────────────────────────┘");
            println!();
        }
    }

    println!("📊 Cycle: {}  |  ⏱️  Update interval: {}s",
             cycle_count, UPDATE_INTERVAL_SECS);
    println!();
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║          🏠  PRESENCE DETECTION AUTOMATION  🏠                    ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    // Setup socket
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();

    println!("📡 UDP socket: {}", socket.local_addr().unwrap());

    // Gateway configuration
    let gateway_ip = [127, 0, 0, 1];
    let gateway_port = 3671;
    let gateway_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), gateway_port);

    println!("🎯 Gateway: {}\n", gateway_addr);

    // Connect to gateway
    println!("--- Connecting to Gateway ---");
    let client = TunnelClient::new(gateway_ip, gateway_port);
    let client = client.connect().unwrap();
    let connect_frame = client.frame_data();

    socket.send_to(connect_frame, gateway_addr).unwrap();
    println!("📤 Sent CONNECT_REQUEST");

    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer).unwrap();

    let mut client = client.handle_connect_response(&buffer[6..len]).unwrap();
    println!("✅ Connected! Channel ID: {}\n", client.channel_id());

    // Setup addresses
    let mut addresses = HashMap::new();

    // Hallway
    addresses.insert("hallway_motion", GroupAddress::from_str("5/0/1").unwrap());
    addresses.insert("hallway_brightness", GroupAddress::from_str("5/1/1").unwrap());
    addresses.insert("hallway_dimmer", GroupAddress::from_str("5/2/1").unwrap());

    // Bathroom
    addresses.insert("bathroom_motion", GroupAddress::from_str("6/0/1").unwrap());
    addresses.insert("bathroom_switch", GroupAddress::from_str("6/2/1").unwrap());
    addresses.insert("bathroom_dimmer", GroupAddress::from_str("6/2/2").unwrap());

    // Office
    addresses.insert("office_motion", GroupAddress::from_str("7/0/1").unwrap());
    addresses.insert("office_brightness", GroupAddress::from_str("7/1/1").unwrap());
    addresses.insert("office_switch", GroupAddress::from_str("7/2/1").unwrap());

    // Staircase
    addresses.insert("staircase_motion", GroupAddress::from_str("8/0/1").unwrap());
    addresses.insert("staircase_switch", GroupAddress::from_str("8/2/1").unwrap());

    // Initialize zone states
    let mut zones = HashMap::new();
    zones.insert("hallway", ZoneState::new("Hallway", HALLWAY_TIMEOUT_SECS, Some(HALLWAY_LUX_THRESHOLD)));
    zones.insert("bathroom", ZoneState::new("Bathroom", BATHROOM_TIMEOUT_SECS, None));
    zones.insert("office", ZoneState::new("Office", OFFICE_TIMEOUT_SECS, Some(OFFICE_LUX_THRESHOLD)));
    zones.insert("staircase", ZoneState::new("Staircase", STAIRCASE_TIMEOUT_SECS, None));

    println!("Starting automation loop...\n");
    println!("Press Ctrl+C to stop\n");

    std::thread::sleep(Duration::from_secs(2));

    let mut cycle_count = 0;

    // Main automation loop
    loop {
        cycle_count += 1;

        // === Read all sensors ===

        // Hallway
        if let Some(motion) = read_motion(&socket, &mut client, gateway_addr,
                                         addresses["hallway_motion"], &mut buffer) {
            zones.get_mut("hallway").unwrap().update_motion(motion);
        }
        if let Some(lux) = read_brightness(&socket, &mut client, gateway_addr,
                                          addresses["hallway_brightness"], &mut buffer) {
            zones.get_mut("hallway").unwrap().update_brightness(lux);
        }

        // Bathroom
        if let Some(motion) = read_motion(&socket, &mut client, gateway_addr,
                                         addresses["bathroom_motion"], &mut buffer) {
            zones.get_mut("bathroom").unwrap().update_motion(motion);
        }

        // Office
        if let Some(motion) = read_motion(&socket, &mut client, gateway_addr,
                                         addresses["office_motion"], &mut buffer) {
            zones.get_mut("office").unwrap().update_motion(motion);
        }
        if let Some(lux) = read_brightness(&socket, &mut client, gateway_addr,
                                          addresses["office_brightness"], &mut buffer) {
            zones.get_mut("office").unwrap().update_brightness(lux);
        }

        // Staircase
        if let Some(motion) = read_motion(&socket, &mut client, gateway_addr,
                                         addresses["staircase_motion"], &mut buffer) {
            zones.get_mut("staircase").unwrap().update_motion(motion);
        }

        // === Apply automation logic ===

        // Hallway automation
        let hallway = zones.get_mut("hallway").unwrap();
        if hallway.should_activate() && !hallway.light_on {
            let level = if is_night_time() { 30 } else { 100 };
            hallway.activate();
            hallway.light_level = level;
            write_dimmer(&socket, &mut client, gateway_addr,
                        addresses["hallway_dimmer"], level, &mut buffer);
        } else if hallway.should_deactivate() {
            hallway.deactivate();
            write_dimmer(&socket, &mut client, gateway_addr,
                        addresses["hallway_dimmer"], 0, &mut buffer);
        }

        // Bathroom automation (with dimming after 1 minute)
        let bathroom = zones.get_mut("bathroom").unwrap();
        if bathroom.should_activate() && !bathroom.light_on {
            bathroom.activate();
            bathroom.light_level = 100;
            write_dimmer(&socket, &mut client, gateway_addr,
                        addresses["bathroom_dimmer"], 100, &mut buffer);
        } else if bathroom.light_on && !bathroom.dimmed_mode {
            if let Some(elapsed) = bathroom.seconds_since_motion() {
                if elapsed >= BATHROOM_DIM_DELAY_SECS {
                    bathroom.dimmed_mode = true;
                    bathroom.light_level = 20;
                    write_dimmer(&socket, &mut client, gateway_addr,
                                addresses["bathroom_dimmer"], 20, &mut buffer);
                }
            }
        } else if bathroom.should_deactivate() {
            bathroom.deactivate();
            write_switch(&socket, &mut client, gateway_addr,
                        addresses["bathroom_switch"], false, &mut buffer);
        }

        // Office automation
        let office = zones.get_mut("office").unwrap();
        if office.should_activate() && !office.light_on {
            office.activate();
            write_switch(&socket, &mut client, gateway_addr,
                        addresses["office_switch"], true, &mut buffer);
        } else if office.should_deactivate() {
            office.deactivate();
            write_switch(&socket, &mut client, gateway_addr,
                        addresses["office_switch"], false, &mut buffer);
        }

        // Staircase automation
        let staircase = zones.get_mut("staircase").unwrap();
        if staircase.should_activate() && !staircase.light_on {
            staircase.activate();
            write_switch(&socket, &mut client, gateway_addr,
                        addresses["staircase_switch"], true, &mut buffer);
        } else if staircase.should_deactivate() {
            staircase.deactivate();
            write_switch(&socket, &mut client, gateway_addr,
                        addresses["staircase_switch"], false, &mut buffer);
        }

        // Display dashboard
        print_dashboard(&zones, cycle_count);

        // Heartbeat every 12 cycles (60 seconds)
        if cycle_count % 12 == 0 {
            println!("💓 Sending heartbeat...");
            let hb_frame = client.send_heartbeat().unwrap();
            socket.send_to(hb_frame, gateway_addr).unwrap();

            let (len, _) = socket.recv_from(&mut buffer).unwrap();
            match client.handle_heartbeat_response(&buffer[6..len]) {
                Ok(c) => client = c,
                Err(_) => {
                    println!("❌ Heartbeat failed - connection lost");
                    break;
                }
            }
        }

        // Wait for next cycle
        std::thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS));
    }

    println!("\n\n❌ Automation terminated");
    println!("System stopped.");
}
