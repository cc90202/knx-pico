//! Complete Home Automation Dashboard Example
//!
//! This example demonstrates a comprehensive home automation system that:
//! - Monitors multiple sensors across different rooms
//! - Controls various devices (lights, blinds, HVAC)
//! - Provides a real-time dashboard display
//! - Handles bidirectional communication (read + write)
//!
//! # Architecture
//!
//! ```
//! Living Room:
//!   - Temperature sensor (1/1/1) - DPT 9.001
//!   - Humidity sensor (1/1/2) - DPT 9.007
//!   - Main light (1/2/1) - DPT 1.001
//!   - Dimmer (1/2/2) - DPT 5.001
//!   - Blind (1/3/1) - DPT 3.008
//!
//! Kitchen:
//!   - Temperature sensor (2/1/1) - DPT 9.001
//!   - Motion sensor (2/0/1) - DPT 1.001
//!   - Light (2/2/1) - DPT 1.001
//!   - Brightness (2/2/2) - DPT 7.013 (lux)
//!
//! Bedroom:
//!   - Temperature sensor (3/1/1) - DPT 9.001
//!   - Window contact (3/0/1) - DPT 1.001
//!   - Light (3/2/1) - DPT 1.001
//!   - Heating valve (3/3/1) - DPT 5.001
//!
//! Energy:
//!   - Total power (4/1/1) - DPT 7.012 (W)
//!   - Energy counter (4/1/2) - DPT 13.010 (Wh)
//! ```
//!
//! # Setup
//!
//! 1. Start the simulator: `python3 knx_simulator.py --verbose`
//! 2. Run this example: `cargo run --example home_automation_dashboard`
//!
//! Or connect to a real KNX gateway by changing the gateway IP below.

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};
use std::str::FromStr;
use std::collections::HashMap;

use knx_rs::protocol::tunnel::TunnelClient;
use knx_rs::addressing::GroupAddress;
use knx_rs::dpt::{Dpt1, Dpt5, Dpt9, DptEncode};

// Update interval for dashboard refresh
const UPDATE_INTERVAL_SECS: u64 = 10;

/// Represents the complete state of the home automation system
#[derive(Debug)]
struct HomeState {
    // Living Room
    living_temp: Option<f32>,
    living_humidity: Option<f32>,
    living_light: bool,
    living_dimmer: u8,  // 0-100%
    living_blind_position: Option<String>,

    // Kitchen
    kitchen_temp: Option<f32>,
    kitchen_motion: bool,
    kitchen_light: bool,
    kitchen_brightness: Option<u16>,  // lux

    // Bedroom
    bedroom_temp: Option<f32>,
    bedroom_window: bool,  // true = open, false = closed
    bedroom_light: bool,
    bedroom_valve: u8,  // 0-100%

    // Energy
    total_power: Option<u16>,  // Watts
    energy_counter: Option<i32>,  // Wh

    // System
    last_update: Instant,
    update_count: u32,
}

impl HomeState {
    fn new() -> Self {
        Self {
            living_temp: None,
            living_humidity: None,
            living_light: false,
            living_dimmer: 0,
            living_blind_position: None,

            kitchen_temp: None,
            kitchen_motion: false,
            kitchen_light: false,
            kitchen_brightness: None,

            bedroom_temp: None,
            bedroom_window: false,
            bedroom_light: false,
            bedroom_valve: 0,

            total_power: None,
            energy_counter: None,

            last_update: Instant::now(),
            update_count: 0,
        }
    }

    fn print_dashboard(&self) {
        // Clear screen (ANSI escape code)
        print!("\x1B[2J\x1B[1;1H");

        println!("╔═══════════════════════════════════════════════════════════════════╗");
        println!("║           🏠  HOME AUTOMATION DASHBOARD  🏠                       ║");
        println!("╚═══════════════════════════════════════════════════════════════════╝");
        println!();

        // Living Room
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ 🛋️  LIVING ROOM                                                  │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│  Temperature:  {}                                  │",
                 format_temp(self.living_temp));
        println!("│  Humidity:     {}                                  │",
                 format_humidity(self.living_humidity));
        println!("│  Main Light:   {}                                              │",
                 format_switch(self.living_light));
        println!("│  Dimmer:       {}                                      │",
                 format_percentage(self.living_dimmer));
        println!("│  Blind:        {}                                      │",
                 self.living_blind_position.as_deref().unwrap_or("Unknown"));
        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();

        // Kitchen
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ 🍳  KITCHEN                                                      │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│  Temperature:  {}                                  │",
                 format_temp(self.kitchen_temp));
        println!("│  Motion:       {}                                          │",
                 format_motion(self.kitchen_motion));
        println!("│  Light:        {}                                              │",
                 format_switch(self.kitchen_light));
        println!("│  Brightness:   {}                                        │",
                 format_lux(self.kitchen_brightness));
        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();

        // Bedroom
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ 🛏️  BEDROOM                                                      │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│  Temperature:  {}                                  │",
                 format_temp(self.bedroom_temp));
        println!("│  Window:       {}                                          │",
                 format_window(self.bedroom_window));
        println!("│  Light:        {}                                              │",
                 format_switch(self.bedroom_light));
        println!("│  Heating:      {}                                      │",
                 format_percentage(self.bedroom_valve));
        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();

        // Energy
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ ⚡ ENERGY                                                        │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│  Current Power:  {}                                      │",
                 format_power(self.total_power));
        println!("│  Total Energy:   {}                                      │",
                 format_energy(self.energy_counter));
        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();

        // System info
        let elapsed = self.last_update.elapsed().as_secs();
        println!("📊 Updates: {}  |  ⏱️  Last update: {}s ago",
                 self.update_count, elapsed);
        println!();
    }

    fn apply_automations(&mut self) -> Vec<AutomationAction> {
        let mut actions = Vec::new();

        // Automation 1: Turn on kitchen light if motion detected and low brightness
        if self.kitchen_motion && self.kitchen_brightness.map(|b| b < 100).unwrap_or(false) {
            if !self.kitchen_light {
                actions.push(AutomationAction::SetLight {
                    room: "Kitchen".to_string(),
                    address: "2/2/1".to_string(),
                    state: true,
                });
            }
        }

        // Automation 2: Close bedroom window if temperature too low
        if let Some(temp) = self.bedroom_temp {
            if temp < 18.0 && self.bedroom_window {
                actions.push(AutomationAction::Alert {
                    message: "⚠️  Bedroom window open with low temperature!".to_string(),
                });
            }
        }

        // Automation 3: Increase bedroom heating if window closed and temp low
        if let Some(temp) = self.bedroom_temp {
            if !self.bedroom_window && temp < 19.0 && self.bedroom_valve < 80 {
                let new_valve = (self.bedroom_valve + 10).min(100);
                actions.push(AutomationAction::SetValve {
                    room: "Bedroom".to_string(),
                    address: "3/3/1".to_string(),
                    position: new_valve,
                });
            }
        }

        // Automation 4: Alert if high power consumption
        if let Some(power) = self.total_power {
            if power > 3000 {
                actions.push(AutomationAction::Alert {
                    message: format!("⚠️  High power consumption: {} W", power),
                });
            }
        }

        actions
    }
}

/// Actions that can be triggered by automation rules
#[derive(Debug)]
enum AutomationAction {
    SetLight { room: String, address: String, state: bool },
    SetValve { room: String, address: String, position: u8 },
    Alert { message: String },
}

// Formatting helpers
fn format_temp(temp: Option<f32>) -> String {
    match temp {
        Some(t) => format!("{:5.1}°C", t),
        None => "  --.-°C".to_string(),
    }
}

fn format_humidity(hum: Option<f32>) -> String {
    match hum {
        Some(h) => format!("{:5.1}%", h),
        None => "  --.-%".to_string(),
    }
}

fn format_switch(on: bool) -> String {
    if on { "🟢 ON ".to_string() } else { "⚫ OFF".to_string() }
}

fn format_percentage(val: u8) -> String {
    format!("{:3}%  {}", val, progress_bar(val))
}

fn format_motion(detected: bool) -> String {
    if detected { "🔴 Detected ".to_string() } else { "⚫ No motion".to_string() }
}

fn format_window(open: bool) -> String {
    if open { "🔓 Open  ".to_string() } else { "🔒 Closed".to_string() }
}

fn format_lux(lux: Option<u16>) -> String {
    match lux {
        Some(l) => format!("{:5} lux", l),
        None => "   -- lux".to_string(),
    }
}

fn format_power(power: Option<u16>) -> String {
    match power {
        Some(p) => format!("{:5} W", p),
        None => "   -- W".to_string(),
    }
}

fn format_energy(energy: Option<i32>) -> String {
    match energy {
        Some(e) if e >= 1000 => format!("{:6.1} kWh", e as f32 / 1000.0),
        Some(e) => format!("{:6} Wh", e),
        None => "    -- Wh".to_string(),
    }
}

fn progress_bar(percentage: u8) -> String {
    let filled = (percentage / 10) as usize;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Read a temperature sensor (DPT 9.001)
fn read_temperature(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    buffer: &mut [u8; 1024],
) -> Option<f32> {
    // Build CEMI GroupValue_Read
    let mut cemi = Vec::new();
    cemi.push(0x11);  // L_Data.req (low priority)
    cemi.push(0x00);  // No additional info
    cemi.push(0xBC);  // Control field 1
    cemi.push(0xE0);  // Control field 2

    // Source address: 1.1.250
    cemi.push(0x11);
    cemi.push(0xFA);

    // Destination group address
    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);

    // NPDU length: 1 byte
    cemi.push(0x01);

    // TPCI/APCI: GroupValue_Read (0x0000)
    cemi.push(0x00);
    cemi.push(0x00);

    // Send request
    if let Ok(frame) = client.send_tunneling_request(&cemi) {
        let _ = socket.send_to(frame, gateway_addr);

        // Wait for ACK
        if let Ok((len, _)) = socket.recv_from(buffer) {
            let _ = client.handle_tunneling_ack(&buffer[6..len]);

            // Try to receive response (with timeout)
            socket.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
            if let Ok((len, _)) = socket.recv_from(buffer) {
                socket.set_read_timeout(Some(Duration::from_secs(2))).ok();

                // Parse CEMI response
                if len > 10 && buffer[6] == 0x06 && buffer[7] == 0x10 {
                    // TUNNELING_REQUEST
                    let cemi_start = 10;
                    if buffer[cemi_start] == 0x29 {  // L_Data.ind
                        // Extract data portion (skip header, get to APCI + data)
                        let data_start = cemi_start + 10;
                        if len > data_start + 2 {
                            let value_bytes = &buffer[data_start+1..data_start+3];
                            if let Ok(temp) = Dpt9::Temperature.decode_from_bytes(&[value_bytes[0], value_bytes[1]]) {
                                return Some(temp);
                            }
                        }
                    }
                }
            }
            socket.set_read_timeout(Some(Duration::from_secs(2))).ok();
        }
    }

    None
}

/// Read a boolean sensor (DPT 1.001)
fn read_bool(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    buffer: &mut [u8; 1024],
) -> Option<bool> {
    // Build CEMI GroupValue_Read
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

/// Read a 16-bit unsigned value (DPT 7.xxx)
fn read_u16(
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

/// Write a boolean value (DPT 1.001)
fn write_bool(
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

/// Write a percentage value (DPT 5.001)
fn write_percentage(
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

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║        🏠  HOME AUTOMATION DASHBOARD - KNX Integration 🏠        ║");
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

    // Define all group addresses
    let addresses = setup_addresses();

    // Initialize state
    let mut state = HomeState::new();
    let mut cycle_count = 0;

    println!("Starting dashboard monitoring...\n");
    println!("Press Ctrl+C to stop\n");

    std::thread::sleep(Duration::from_secs(2));

    // Main monitoring loop
    loop {
        cycle_count += 1;

        // === Read all sensors ===

        // Living Room
        state.living_temp = read_temperature(&socket, &mut client, gateway_addr,
                                             addresses["living_temp"], &mut buffer);
        state.living_humidity = read_temperature(&socket, &mut client, gateway_addr,
                                                 addresses["living_humidity"], &mut buffer);
        state.living_light = read_bool(&socket, &mut client, gateway_addr,
                                       addresses["living_light"], &mut buffer).unwrap_or(false);

        // Kitchen
        state.kitchen_temp = read_temperature(&socket, &mut client, gateway_addr,
                                              addresses["kitchen_temp"], &mut buffer);
        state.kitchen_motion = read_bool(&socket, &mut client, gateway_addr,
                                        addresses["kitchen_motion"], &mut buffer).unwrap_or(false);
        state.kitchen_light = read_bool(&socket, &mut client, gateway_addr,
                                       addresses["kitchen_light"], &mut buffer).unwrap_or(false);
        state.kitchen_brightness = read_u16(&socket, &mut client, gateway_addr,
                                           addresses["kitchen_brightness"], &mut buffer);

        // Bedroom
        state.bedroom_temp = read_temperature(&socket, &mut client, gateway_addr,
                                             addresses["bedroom_temp"], &mut buffer);
        state.bedroom_window = read_bool(&socket, &mut client, gateway_addr,
                                        addresses["bedroom_window"], &mut buffer).unwrap_or(false);
        state.bedroom_light = read_bool(&socket, &mut client, gateway_addr,
                                       addresses["bedroom_light"], &mut buffer).unwrap_or(false);

        // Energy
        state.total_power = read_u16(&socket, &mut client, gateway_addr,
                                    addresses["total_power"], &mut buffer);

        // === Apply automation rules ===
        let actions = state.apply_automations();
        for action in actions {
            match action {
                AutomationAction::SetLight { room, address, state: light_state } => {
                    println!("🤖 Automation: Turning {} light {}", room,
                            if light_state { "ON" } else { "OFF" });
                    let addr = GroupAddress::from_str(&address).unwrap();
                    write_bool(&socket, &mut client, gateway_addr, addr, light_state, &mut buffer);
                }
                AutomationAction::SetValve { room, address, position } => {
                    println!("🤖 Automation: Setting {} valve to {}%", room, position);
                    let addr = GroupAddress::from_str(&address).unwrap();
                    write_percentage(&socket, &mut client, gateway_addr, addr, position, &mut buffer);
                }
                AutomationAction::Alert { message } => {
                    println!("{}", message);
                }
            }
        }

        // Update state
        state.last_update = Instant::now();
        state.update_count = cycle_count;

        // Display dashboard
        state.print_dashboard();

        // Heartbeat every 6 cycles (60 seconds)
        if cycle_count % 6 == 0 {
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

    println!("\n\n❌ Dashboard terminated");
    println!("Monitoring stopped.");
}

/// Setup all KNX group addresses used in the system
fn setup_addresses() -> HashMap<&'static str, GroupAddress> {
    let mut map = HashMap::new();

    // Living Room
    map.insert("living_temp", GroupAddress::new(1, 1, 1).unwrap());
    map.insert("living_humidity", GroupAddress::new(1, 1, 2).unwrap());
    map.insert("living_light", GroupAddress::new(1, 2, 1).unwrap());
    map.insert("living_dimmer", GroupAddress::new(1, 2, 2).unwrap());
    map.insert("living_blind", GroupAddress::new(1, 3, 1).unwrap());

    // Kitchen
    map.insert("kitchen_temp", GroupAddress::new(2, 1, 1).unwrap());
    map.insert("kitchen_motion", GroupAddress::new(2, 0, 1).unwrap());
    map.insert("kitchen_light", GroupAddress::new(2, 2, 1).unwrap());
    map.insert("kitchen_brightness", GroupAddress::new(2, 2, 2).unwrap());

    // Bedroom
    map.insert("bedroom_temp", GroupAddress::new(3, 1, 1).unwrap());
    map.insert("bedroom_window", GroupAddress::new(3, 0, 1).unwrap());
    map.insert("bedroom_light", GroupAddress::new(3, 2, 1).unwrap());
    map.insert("bedroom_valve", GroupAddress::new(3, 3, 1).unwrap());

    // Energy
    map.insert("total_power", GroupAddress::new(4, 1, 1).unwrap());
    map.insert("energy_counter", GroupAddress::new(4, 1, 2).unwrap());

    map
}
