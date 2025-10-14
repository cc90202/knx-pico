//! Complete example: Send dimming and blind control commands via KNX
//!
//! This example demonstrates how to use DPT 3.xxx with TunnelClient
//! to control lights (dimming) and blinds in a real KNX installation.
//!
//! # Setup
//!
//! Before running:
//! 1. Start the simulator: `python3 knx_simulator.py --verbose`
//! 2. Run this example: `cargo run --example dimming_control_client`
//!
//! Or connect to a real KNX gateway by changing the gateway IP below.

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use std::thread;

use knx_rs::protocol::tunnel::TunnelClient;
use knx_rs::addressing::GroupAddress;
use knx_rs::dpt::{Dpt3, StepCode};

fn main() {
    println!("=== KNX Dimming & Blind Control Example ===\n");

    // =========================================================================
    // Setup: Create socket and connect to gateway
    // =========================================================================

    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();

    println!("📡 UDP socket: {}", socket.local_addr().unwrap());

    // Gateway configuration
    // Change this to your KNX gateway IP for real hardware
    let gateway_ip = [127, 0, 0, 1];  // localhost for simulator
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

    // =========================================================================
    // Scenario 1: Living Room - Dimming Control
    // =========================================================================

    println!("=== Scenario 1: Living Room Dimming ===\n");

    // Group address for living room light
    let living_room_light = GroupAddress::new(1, 2, 10).unwrap();  // 1/2/10
    println!("💡 Living Room Light: {}", living_room_light);

    // Start dimming up (increase brightness)
    println!("\n1. Start dimming UP (increase by 4 intervals)");
    let dim_up = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals4).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, living_room_light, &[dim_up], &mut buffer);

    // Simulate button hold (in real app, this would be triggered by button events)
    thread::sleep(Duration::from_millis(500));

    // Stop dimming
    println!("\n2. Stop dimming (break)");
    let dim_stop = Dpt3::Dimming.encode_to_byte(false, StepCode::Break).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, living_room_light, &[dim_stop], &mut buffer);

    thread::sleep(Duration::from_millis(300));

    // Start dimming down (decrease brightness)
    println!("\n3. Start dimming DOWN (decrease by 2 intervals)");
    let dim_down = Dpt3::Dimming.encode_to_byte(false, StepCode::Intervals2).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, living_room_light, &[dim_down], &mut buffer);

    thread::sleep(Duration::from_millis(500));

    // Stop dimming
    println!("\n4. Stop dimming (break)");
    send_group_write(&socket, &mut client, gateway_addr, living_room_light, &[dim_stop], &mut buffer);

    // =========================================================================
    // Scenario 2: Bedroom - Blind Control
    // =========================================================================

    println!("\n\n=== Scenario 2: Bedroom Blind Control ===\n");

    // Group address for bedroom blind
    let bedroom_blind = GroupAddress::new(2, 1, 5).unwrap();  // 2/1/5
    println!("🪟 Bedroom Blind: {}", bedroom_blind);

    // Morning: Open blinds (move up)
    println!("\n1. Morning routine - Open blinds (UP, 32 intervals)");
    let blind_up = Dpt3::Blind.encode_to_byte(false, StepCode::Intervals32).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, bedroom_blind, &[blind_up], &mut buffer);

    thread::sleep(Duration::from_secs(1));

    // Stop blind
    println!("\n2. Blinds fully open - Stop");
    let blind_stop = Dpt3::Blind.encode_to_byte(false, StepCode::Break).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, bedroom_blind, &[blind_stop], &mut buffer);

    thread::sleep(Duration::from_millis(300));

    // Evening: Close blinds (move down)
    println!("\n3. Evening routine - Close blinds (DOWN, 64 intervals)");
    let blind_down = Dpt3::Blind.encode_to_byte(true, StepCode::Intervals64).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, bedroom_blind, &[blind_down], &mut buffer);

    thread::sleep(Duration::from_secs(1));

    // Stop blind
    println!("\n4. Blinds fully closed - Stop");
    let blind_stop_down = Dpt3::Blind.encode_to_byte(true, StepCode::Break).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, bedroom_blind, &[blind_stop_down], &mut buffer);

    // =========================================================================
    // Scenario 3: Office - Multiple Lights
    // =========================================================================

    println!("\n\n=== Scenario 3: Office - Multiple Lights ===\n");

    let office_light_1 = GroupAddress::new(3, 1, 1).unwrap();  // 3/1/1
    let office_light_2 = GroupAddress::new(3, 1, 2).unwrap();  // 3/1/2

    println!("💡 Office Light 1: {}", office_light_1);
    println!("💡 Office Light 2: {}", office_light_2);

    // Dim both lights up simultaneously
    println!("\n1. Dim both office lights UP (8 intervals)");
    let dim_office = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals8).unwrap();

    send_group_write(&socket, &mut client, gateway_addr, office_light_1, &[dim_office], &mut buffer);
    send_group_write(&socket, &mut client, gateway_addr, office_light_2, &[dim_office], &mut buffer);

    thread::sleep(Duration::from_millis(800));

    // Stop both lights
    println!("\n2. Stop dimming both lights");
    let stop = Dpt3::Dimming.encode_to_byte(false, StepCode::Break).unwrap();

    send_group_write(&socket, &mut client, gateway_addr, office_light_1, &[stop], &mut buffer);
    send_group_write(&socket, &mut client, gateway_addr, office_light_2, &[stop], &mut buffer);

    // =========================================================================
    // Scenario 4: Long Press Simulation
    // =========================================================================

    println!("\n\n=== Scenario 4: Long Press Simulation ===\n");
    println!("Simulating user holding dimmer button...\n");

    let kitchen_light = GroupAddress::new(1, 3, 20).unwrap();  // 1/3/20
    println!("💡 Kitchen Light: {}", kitchen_light);

    // Start dimming
    println!("1. Button PRESSED → Start dimming UP (1 interval)");
    let dim_start = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals1).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, kitchen_light, &[dim_start], &mut buffer);

    // Simulate holding (send periodic updates)
    for i in 1..=5 {
        thread::sleep(Duration::from_millis(200));
        println!("   ...holding... ({})", i);
        // In a real system, you'd keep sending the same command periodically
        send_group_write(&socket, &mut client, gateway_addr, kitchen_light, &[dim_start], &mut buffer);
    }

    // Release button
    println!("\n2. Button RELEASED → Stop dimming");
    let dim_stop = Dpt3::Dimming.encode_to_byte(false, StepCode::Break).unwrap();
    send_group_write(&socket, &mut client, gateway_addr, kitchen_light, &[dim_stop], &mut buffer);

    // =========================================================================
    // Cleanup: Disconnect
    // =========================================================================

    println!("\n\n--- Disconnecting ---");
    let client = client.disconnect().unwrap();
    let disc_frame = client.frame_data();
    socket.send_to(disc_frame, gateway_addr).unwrap();
    println!("📤 Sent DISCONNECT_REQUEST");

    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    let _client = client.finish(&buffer[6..len]).unwrap();
    println!("✅ Disconnected\n");

    // =========================================================================
    // Summary
    // =========================================================================

    println!("🎉 ALL SCENARIOS COMPLETED!\n");
    println!("Summary:");
    println!("  ✅ Scenario 1: Living room dimming (up/down/stop)");
    println!("  ✅ Scenario 2: Bedroom blind control (open/close)");
    println!("  ✅ Scenario 3: Multiple office lights");
    println!("  ✅ Scenario 4: Long press simulation");
    println!("\nDPT 3.xxx commands sent successfully via KNXnet/IP Tunneling!");
}

/// Helper function to send a GroupValue_Write command
///
/// This builds a complete CEMI frame with the DPT data and sends it
/// through the tunnel connection.
fn send_group_write(
    socket: &UdpSocket,
    client: &mut TunnelClient<knx_rs::protocol::tunnel::Connected>,
    gateway_addr: SocketAddrV4,
    group_addr: GroupAddress,
    data: &[u8],
    buffer: &mut [u8; 1024],
) {
    // Build CEMI frame: L_Data.req with GroupValue_Write
    let mut cemi = Vec::new();

    // CEMI header
    cemi.push(0x29);  // Message code: L_Data.req
    cemi.push(0x00);  // Additional info length

    // Control fields (standard frame, no repeat, broadcast, normal priority, no ack, no error)
    cemi.push(0xBC);  // Control field 1
    cemi.push(0xE0);  // Control field 2

    // Source address: 1.1.250 (0x11FA)
    cemi.push(0x11);
    cemi.push(0xFA);

    // Destination group address
    let dest_raw = group_addr.raw();
    cemi.push((dest_raw >> 8) as u8);
    cemi.push((dest_raw & 0xFF) as u8);

    // NPDU length (1 + data length)
    cemi.push((1 + data.len()) as u8);

    // TPCI/APCI: GroupValue_Write (0x0080 with 6-bit value in lower bits)
    // For multi-byte data, we set bit pattern 00 in TPCI and 0x80 in APCI
    cemi.push(0x00);  // TPCI

    if data.len() == 1 && data[0] <= 0x3F {
        // Small data (≤6 bits): encode in APCI
        cemi.push(0x80 | data[0]);
    } else {
        // Large data: separate APCI and data bytes
        cemi.push(0x80);  // APCI: GroupValue_Write
        cemi.extend_from_slice(data);
    }

    // Send TUNNELING_REQUEST with CEMI
    let tunnel_frame = client.send_tunneling_request(&cemi).unwrap();
    socket.send_to(tunnel_frame, gateway_addr).unwrap();

    println!("   📤 Sent: {} → 0x{:02X} (seq={})",
             group_addr, data[0], client.send_sequence() - 1);

    // Wait for ACK
    let (len, _) = socket.recv_from(buffer).unwrap();
    client.handle_tunneling_ack(&buffer[6..len]).unwrap();
    println!("   ✅ ACK received");
}
