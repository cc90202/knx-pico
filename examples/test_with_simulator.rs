//! Test TunnelClient with the virtual KNX gateway simulator
//!
//! Before running this example:
//! 1. Start the simulator: `python3 knx_simulator.py --verbose`
//! 2. Run this example: `cargo run --example test_with_simulator`

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use knx_rs::protocol::tunnel::TunnelClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TunnelClient Test with Simulator ===\n");

    // Create UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;

    println!("📡 UDP socket bound to: {}", socket.local_addr()?);

    // Gateway address (simulator)
    let gateway_ip = [127, 0, 0, 1];
    let gateway_port = 3671;
    let gateway_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), gateway_port);
    println!("🎯 Gateway: {}\n", gateway_addr);

    // === Test 1: Connection ===
    println!("--- Test 1: Connection ---");

    // Create client in Idle state
    let client = TunnelClient::new(gateway_ip, gateway_port);
    println!("✅ Created TunnelClient in Idle state");

    // Connect (Idle → Connecting)
    let (client, connect_frame) = client.connect()?;
    println!("✅ Built CONNECT_REQUEST ({} bytes)", connect_frame.len());

    // Send CONNECT_REQUEST
    socket.send_to(connect_frame, gateway_addr)?;
    println!("📤 Sent CONNECT_REQUEST");

    // Receive CONNECT_RESPONSE
    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer)?;
    println!("📥 Received CONNECT_RESPONSE ({} bytes)", len);

    // Parse response (Connecting → Connected)
    let mut client = client.handle_connect_response(&buffer[6..len])?;
    println!("✅ Connected! Channel ID: {}\n", client.channel_id());

    // === Test 2: Send Tunneling Request ===
    println!("--- Test 2: Tunneling Request ---");

    // Build a simple cEMI frame (GroupValue_Write to 1/2/3 with value ON)
    let cemi_data = [
        0x29, // Message code: L_Data.req
        0x00, // Additional info length
        0xBC, 0xE0, // Control fields
        0x11, 0xFA, // Source: 1.1.250
        0x0A, 0x03, // Destination: 1/2/3
        0x01, // NPDU length
        0x00, 0x81, // TPCI/APCI: GroupValue_Write
    ];

    let tunnel_frame = client.send_tunneling_request(&cemi_data)?;
    println!("✅ Built TUNNELING_REQUEST ({} bytes)", tunnel_frame.len());

    // Send TUNNELING_REQUEST
    socket.send_to(tunnel_frame, gateway_addr)?;
    println!("📤 Sent TUNNELING_REQUEST (seq=0)");

    // Receive TUNNELING_ACK
    let (len, _) = socket.recv_from(&mut buffer)?;
    println!("📥 Received TUNNELING_ACK ({} bytes)", len);

    // Handle ACK
    client.handle_tunneling_ack(&buffer[6..len])?;
    println!("✅ ACK verified (status OK)");
    println!("✅ Send sequence incremented to: {}\n", client.send_sequence());

    // === Test 3: Heartbeat ===
    println!("--- Test 3: Heartbeat ---");

    let heartbeat_frame = client.send_heartbeat()?;
    println!("✅ Built CONNECTIONSTATE_REQUEST ({} bytes)", heartbeat_frame.len());

    socket.send_to(heartbeat_frame, gateway_addr)?;
    println!("📤 Sent CONNECTIONSTATE_REQUEST");

    let (len, _) = socket.recv_from(&mut buffer)?;
    println!("📥 Received CONNECTIONSTATE_RESPONSE ({} bytes)", len);

    let client = client.handle_heartbeat_response(&buffer[6..len])?;
    println!("✅ Connection still alive\n");

    // === Test 4: Disconnect ===
    println!("--- Test 4: Disconnect ---");

    let (client, disc_frame) = client.disconnect()?;
    println!("✅ Built DISCONNECT_REQUEST ({} bytes)", disc_frame.len());

    socket.send_to(disc_frame, gateway_addr)?;
    println!("📤 Sent DISCONNECT_REQUEST");

    let (len, _) = socket.recv_from(&mut buffer)?;
    println!("📥 Received DISCONNECT_RESPONSE ({} bytes)", len);

    let _client = client.finish(&buffer[6..len])?;
    println!("✅ Disconnected, back to Idle state\n");

    // === Summary ===
    println!("🎉 ALL TESTS PASSED!");
    println!("\nTypestate transitions verified:");
    println!("  Idle → Connecting → Connected → Disconnecting → Idle");

    Ok(())
}
