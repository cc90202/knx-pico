//! Smart Thermostat - Temperature Control with KNX
//!
//! This example demonstrates a complete temperature control system:
//! - Reads temperature from KNX sensor (DPT 9.001)
//! - Compares with desired setpoint
//! - Controls heating valve/boiler (DPT 5.001 percentage or DPT 1.001 on/off)
//! - Implements simple PI control loop
//! - Logs temperature history
//! - Provides real-time dashboard
//!
//! # Hardware Setup
//!
//! Required KNX devices:
//! - Temperature sensor at group address 1/1/1 (DPT 9.001)
//! - Heating valve at group address 1/2/1 (DPT 5.001 - 0-100%)
//! - Optional: Room presence sensor at 1/0/1 (DPT 1.001)
//!
//! # Running
//!
//! With simulator:
//! ```bash
//! python3 knx_simulator.py --verbose
//! cargo run --example smart_thermostat
//! ```
//!
//! With real gateway:
//! Change GATEWAY_IP to your KNX gateway address.

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::str::FromStr;

use knx_rs::protocol::tunnel::TunnelClient;
use knx_rs::addressing::GroupAddress;
use knx_rs::dpt::{Dpt5, Dpt9};

// Configuration
const GATEWAY_IP: [u8; 4] = [127, 0, 0, 1];  // Change to your gateway IP
const GATEWAY_PORT: u16 = 3671;

// Group Addresses
const TEMP_SENSOR_ADDR: &str = "1/1/1";      // Temperature sensor (DPT 9.001)
const HEATING_VALVE_ADDR: &str = "1/2/1";    // Heating valve (DPT 5.001 - percentage)
const PRESENCE_SENSOR_ADDR: &str = "1/0/1";  // Room presence (DPT 1.001)

// Control Parameters
const SETPOINT_TEMP: f32 = 21.0;             // Target temperature (°C)
const HYSTERESIS: f32 = 0.5;                  // Temperature dead band (±0.5°C)
const KP: f32 = 10.0;                         // Proportional gain
const KI: f32 = 0.5;                          // Integral gain
const UPDATE_INTERVAL_SECS: u64 = 30;        // Read temperature every 30 seconds
const MAX_VALVE_CHANGE: u8 = 10;             // Max valve change per cycle (%)

/// Thermostat state
struct ThermostatState {
    current_temp: Option<f32>,
    setpoint: f32,
    valve_position: u8,  // 0-100%
    room_occupied: bool,
    temperature_history: VecDeque<(Instant, f32)>,
    integral_error: f32,
    last_update: Instant,
}

impl ThermostatState {
    fn new(setpoint: f32) -> Self {
        Self {
            current_temp: None,
            setpoint,
            valve_position: 0,
            room_occupied: false,
            temperature_history: VecDeque::with_capacity(100),
            integral_error: 0.0,
            last_update: Instant::now(),
        }
    }

    fn add_temperature(&mut self, temp: f32) {
        self.current_temp = Some(temp);
        self.temperature_history.push_back((Instant::now(), temp));

        // Keep only last 100 readings
        if self.temperature_history.len() > 100 {
            self.temperature_history.pop_front();
        }
    }

    fn calculate_valve_position(&mut self) -> u8 {
        let temp = match self.current_temp {
            Some(t) => t,
            None => return 0, // No temperature data, close valve
        };

        // If room is not occupied, use lower setpoint (energy saving)
        let effective_setpoint = if self.room_occupied {
            self.setpoint
        } else {
            self.setpoint - 2.0  // 2°C lower when unoccupied
        };

        // Calculate error
        let error = effective_setpoint - temp;

        // Check hysteresis (dead band)
        if error.abs() < HYSTERESIS && self.valve_position < 50 {
            return self.valve_position; // Within dead band, keep current position
        }

        // PI Controller
        // P term: proportional to error
        let p_term = KP * error;

        // I term: integral of error over time
        let dt = self.last_update.elapsed().as_secs_f32();
        self.integral_error += error * dt;
        // Anti-windup: limit integral term
        self.integral_error = self.integral_error.clamp(-50.0, 50.0);
        let i_term = KI * self.integral_error;

        self.last_update = Instant::now();

        // Calculate new valve position
        let control_output = p_term + i_term;
        let new_position = (control_output).clamp(0.0, 100.0) as u8;

        // Limit rate of change (smooth transitions)
        let current = self.valve_position as i16;
        let target = new_position as i16;
        let delta = (target - current).clamp(-(MAX_VALVE_CHANGE as i16), MAX_VALVE_CHANGE as i16);

        ((current + delta) as u8).clamp(0, 100)
    }

    fn print_status(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║           SMART THERMOSTAT STATUS                        ║");
        println!("╠═══════════════════════════════════════════════════════════╣");

        if let Some(temp) = self.current_temp {
            println!("║ Current Temperature: {:.1}°C                             ║", temp);
        } else {
            println!("║ Current Temperature: --.-°C (no data)                    ║");
        }

        let target = if self.room_occupied {
            self.setpoint
        } else {
            self.setpoint - 2.0
        };

        println!("║ Target Temperature:  {:.1}°C                             ║", target);
        println!("║ Heating Valve:       {}%                                 ║", self.valve_position);
        println!("║ Room Occupied:       {}                                  ║",
                 if self.room_occupied { "YES" } else { "NO " });
        println!("║                                                           ║");

        if let Some(temp) = self.current_temp {
            let error = target - temp;
            let status = if error > HYSTERESIS {
                "HEATING 🔥"
            } else if error < -HYSTERESIS {
                "COOLING ❄️ "
            } else {
                "OK ✓     "
            };
            println!("║ Status: {} (error: {:+.1}°C)                          ║", status, error);
        }

        println!("╚═══════════════════════════════════════════════════════════╝\n");

        // Print recent history
        if self.temperature_history.len() >= 2 {
            println!("Recent temperature history:");
            for (instant, temp) in self.temperature_history.iter().rev().take(5) {
                let elapsed = instant.elapsed().as_secs();
                println!("  {}s ago: {:.1}°C", elapsed, temp);
            }
            println!();
        }
    }
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        KNX SMART THERMOSTAT - Temperature Control         ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Parse group addresses
    let temp_sensor = GroupAddress::from_str(TEMP_SENSOR_ADDR).unwrap();
    let heating_valve = GroupAddress::from_str(HEATING_VALVE_ADDR).unwrap();
    let presence_sensor = GroupAddress::from_str(PRESENCE_SENSOR_ADDR).unwrap();

    println!("Configuration:");
    println!("  Temperature Sensor: {}", temp_sensor);
    println!("  Heating Valve:      {}", heating_valve);
    println!("  Presence Sensor:    {}", presence_sensor);
    println!("  Target Temp:        {:.1}°C", SETPOINT_TEMP);
    println!("  Update Interval:    {}s\n", UPDATE_INTERVAL_SECS);

    // Setup socket and connection
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();

    let gateway_addr = SocketAddrV4::new(Ipv4Addr::from(GATEWAY_IP), GATEWAY_PORT);
    println!("Connecting to gateway: {}", gateway_addr);

    // Connect
    let client = TunnelClient::new(GATEWAY_IP, GATEWAY_PORT);
    let client = client.connect().unwrap();
    socket.send_to(client.frame_data(), gateway_addr).unwrap();

    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    let mut client = client.handle_connect_response(&buffer[6..len]).unwrap();

    println!("✅ Connected! Channel ID: {}\n", client.channel_id());

    // Initialize thermostat state
    let mut state = ThermostatState::new(SETPOINT_TEMP);
    let mut _last_read_time = Instant::now();
    let mut cycle_count = 0;

    println!("Starting control loop...\n");

    // Main control loop
    loop {
        cycle_count += 1;
        println!("=== Cycle {} ===", cycle_count);

        // 1. Read current temperature
        println!("📖 Reading temperature sensor...");
        if let Some(temp) = read_temperature(&socket, &mut client, gateway_addr, temp_sensor, &mut buffer) {
            println!("   Temperature: {:.1}°C", temp);
            state.add_temperature(temp);
        } else {
            println!("   ⚠️  Failed to read temperature");
        }

        // 2. Read presence sensor (optional)
        println!("📖 Reading presence sensor...");
        if let Some(occupied) = read_presence(&socket, &mut client, gateway_addr, presence_sensor, &mut buffer) {
            state.room_occupied = occupied;
            println!("   Room: {}", if occupied { "OCCUPIED" } else { "EMPTY" });
        } else {
            println!("   ⚠️  Failed to read presence");
        }

        // 3. Calculate new valve position
        let old_valve = state.valve_position;
        let new_valve = state.calculate_valve_position();
        state.valve_position = new_valve;

        if old_valve != new_valve {
            println!("🔧 Adjusting heating valve: {}% → {}%", old_valve, new_valve);
            write_valve_position(&socket, &mut client, gateway_addr, heating_valve, new_valve, &mut buffer);
        } else {
            println!("✓  Valve position unchanged: {}%", new_valve);
        }

        // 4. Send heartbeat every few cycles
        if cycle_count % 3 == 0 {
            println!("💓 Sending heartbeat...");
            let hb_frame = client.send_heartbeat().unwrap();
            socket.send_to(hb_frame, gateway_addr).unwrap();
            let (len, _) = socket.recv_from(&mut buffer).unwrap();
            let client_result = client.handle_heartbeat_response(&buffer[6..len]);
            match client_result {
                Ok(c) => client = c,
                Err(_) => {
                    println!("❌ Heartbeat failed, reconnecting...");
                    break;
                }
            }
        }

        // 5. Display status
        state.print_status();

        // 6. Wait for next cycle
        println!("⏳ Waiting {} seconds until next cycle...\n", UPDATE_INTERVAL_SECS);
        std::thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS));

        _last_read_time = Instant::now();
    }

    // Loop exited due to heartbeat failure or error
    println!("\n\n❌ Control loop terminated");
    println!("Thermostat stopped. Restart to reconnect.");
}

/// Read temperature from KNX sensor
fn read_temperature(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    sensor_addr: GroupAddress,
    buffer: &mut [u8; 1024],
) -> Option<f32> {
    // Send GroupValue_Read
    let cemi = build_group_read(sensor_addr);
    let frame = client.send_tunneling_request(&cemi).ok()?;
    socket.send_to(frame, gateway_addr).ok()?;

    // Wait for ACK
    let (len, _) = socket.recv_from(buffer).ok()?;
    client.handle_tunneling_ack(&buffer[6..len]).ok()?;

    // Wait for response (TUNNELING_INDICATION with GroupValue_Response)
    let (len, _) = socket.recv_from(buffer).ok()?;

    // Parse CEMI data
    let cemi_data = client.handle_tunneling_indication(&buffer[6..len]).ok()?;

    // Send ACK for the indication
    let ack_frame = client.build_tunneling_ack(client.recv_sequence() - 1, 0).ok()?;
    socket.send_to(ack_frame, gateway_addr).ok()?;

    // Extract temperature value from CEMI
    // CEMI format: [msg_code, add_info_len, ctrl1, ctrl2, src_hi, src_lo, dest_hi, dest_lo, npdu_len, tpci, apci, data...]
    if cemi_data.len() >= 11 {
        // Temperature is in last 2 bytes (DPT 9.001)
        let temp_data = &cemi_data[cemi_data.len() - 2..];
        Dpt9::Temperature.decode_from_bytes(temp_data).ok()
    } else {
        None
    }
}

/// Read presence sensor
fn read_presence(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    sensor_addr: GroupAddress,
    buffer: &mut [u8; 1024],
) -> Option<bool> {
    // Send GroupValue_Read
    let cemi = build_group_read(sensor_addr);
    let frame = client.send_tunneling_request(&cemi).ok()?;
    socket.send_to(frame, gateway_addr).ok()?;

    // Wait for ACK
    let (len, _) = socket.recv_from(buffer).ok()?;
    client.handle_tunneling_ack(&buffer[6..len]).ok()?;

    // Wait for response
    let (len, _) = socket.recv_from(buffer).ok()?;
    let cemi_data = client.handle_tunneling_indication(&buffer[6..len]).ok()?;

    // Send ACK
    let ack_frame = client.build_tunneling_ack(client.recv_sequence() - 1, 0).ok()?;
    socket.send_to(ack_frame, gateway_addr).ok()?;

    // Extract boolean value (last byte, LSB)
    if cemi_data.len() >= 10 {
        let value_byte = cemi_data[cemi_data.len() - 1];
        Some((value_byte & 0x01) != 0)
    } else {
        None
    }
}

/// Write valve position (0-100%)
fn write_valve_position(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    valve_addr: GroupAddress,
    percentage: u8,
    buffer: &mut [u8; 1024],
) {
    let valve_byte = Dpt5::Percentage.encode_to_byte(percentage as u16).unwrap();
    let cemi = build_group_write(valve_addr, &[valve_byte]);

    let frame = client.send_tunneling_request(&cemi).unwrap();
    socket.send_to(frame, gateway_addr).unwrap();

    // Wait for ACK
    let (len, _) = socket.recv_from(buffer).unwrap();
    client.handle_tunneling_ack(&buffer[6..len]).unwrap();
}

/// Build GroupValue_Read CEMI frame
fn build_group_read(group_addr: GroupAddress) -> Vec<u8> {
    let mut cemi = Vec::new();
    cemi.push(0x29);  // L_Data.req
    cemi.push(0x00);  // No additional info
    cemi.push(0xBC);  // Control field 1
    cemi.push(0xE0);  // Control field 2
    cemi.push(0x11);  // Source: 1.1.250
    cemi.push(0xFA);
    let dest = group_addr.raw();
    cemi.push((dest >> 8) as u8);
    cemi.push((dest & 0xFF) as u8);
    cemi.push(0x01);  // NPDU length
    cemi.push(0x00);  // TPCI
    cemi.push(0x00);  // APCI: GroupValue_Read
    cemi
}

/// Build GroupValue_Write CEMI frame
fn build_group_write(group_addr: GroupAddress, data: &[u8]) -> Vec<u8> {
    let mut cemi = Vec::new();
    cemi.push(0x29);  // L_Data.req
    cemi.push(0x00);  // No additional info
    cemi.push(0xBC);  // Control field 1
    cemi.push(0xE0);  // Control field 2
    cemi.push(0x11);  // Source: 1.1.250
    cemi.push(0xFA);
    let dest = group_addr.raw();
    cemi.push((dest >> 8) as u8);
    cemi.push((dest & 0xFF) as u8);
    cemi.push((1 + data.len()) as u8);  // NPDU length
    cemi.push(0x00);  // TPCI

    if data.len() == 1 && data[0] <= 0x3F {
        cemi.push(0x80 | data[0]);  // APCI with 6-bit data
    } else {
        cemi.push(0x80);  // APCI: GroupValue_Write
        cemi.extend_from_slice(data);
    }
    cemi
}
