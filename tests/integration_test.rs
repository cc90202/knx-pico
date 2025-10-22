//! Integration tests for knx-rs library
//!
//! These tests verify that the library works correctly with the KNX simulator.
//!
//! ## Running Tests
//!
//! ```bash
//! # Terminal 1: Start simulator
//! python3 knx_simulator.py --verbose
//!
//! # Terminal 2: Run tests
//! cargo test --test integration_test -- --ignored
//! ```
//!
//! Note: Tests are marked with #[ignore] to avoid running them in CI without simulator.

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::time::Duration;

// Only import types from the library crate
use knx_rs::addressing::{GroupAddress, IndividualAddress};
use knx_rs::protocol::tunnel::TunnelClient;

const SIMULATOR_IP: [u8; 4] = [127, 0, 0, 1];
const SIMULATOR_PORT: u16 = 3671;
const TEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Helper to create a UDP socket for testing
fn create_test_socket() -> std::io::Result<UdpSocket> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(TEST_TIMEOUT))?;
    socket.set_write_timeout(Some(TEST_TIMEOUT))?;
    Ok(socket)
}

/// Helper to get simulator address
fn simulator_addr() -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::from(SIMULATOR_IP), SIMULATOR_PORT)
}

#[test]
#[ignore] // Run with: cargo test --test integration_test -- --ignored
fn test_tunnel_connection() {
    println!("\n=== Test: Tunnel Connection ===");

    let socket = create_test_socket().expect("Failed to create socket");
    println!("✓ Socket created: {}", socket.local_addr().unwrap());

    // Create client
    let client = TunnelClient::new(SIMULATOR_IP, SIMULATOR_PORT);
    println!("✓ TunnelClient created");

    // Connect
    let (client, connect_frame) = client.connect().expect("Failed to create CONNECT_REQUEST");
    println!("✓ CONNECT_REQUEST built ({} bytes)", connect_frame.len());

    // Send CONNECT_REQUEST
    socket.send_to(connect_frame, simulator_addr()).expect("Failed to send");
    println!("✓ CONNECT_REQUEST sent");

    // Receive CONNECT_RESPONSE
    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer).expect("No CONNECT_RESPONSE received");
    println!("✓ CONNECT_RESPONSE received ({} bytes)", len);

    // Parse response
    let client = client.handle_connect_response(&buffer[6..len])
        .expect("Failed to parse CONNECT_RESPONSE");
    println!("✓ Connected! Channel ID: {}", client.channel_id());

    assert!(client.channel_id() > 0, "Channel ID should be non-zero");
}

#[test]
#[ignore]
fn test_tunnel_send_cemi() {
    println!("\n=== Test: Send CEMI Frame ===");

    let socket = create_test_socket().expect("Failed to create socket");

    // Connect first
    let client = TunnelClient::new(SIMULATOR_IP, SIMULATOR_PORT);
    let (client, connect_frame) = client.connect().unwrap();
    socket.send_to(connect_frame, simulator_addr()).unwrap();

    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    let mut client = client.handle_connect_response(&buffer[6..len]).unwrap();
    println!("✓ Connected with channel ID: {}", client.channel_id());

    // Build a simple CEMI frame (GroupValue_Write)
    let cemi_frame = build_test_cemi_frame();
    println!("✓ CEMI frame built ({} bytes)", cemi_frame.len());

    // Send CEMI
    let tunneling_request = client.send_cemi(&cemi_frame).unwrap();
    socket.send_to(&tunneling_request, simulator_addr()).unwrap();
    println!("✓ TUNNELING_REQUEST sent");

    // Receive TUNNELING_ACK
    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    println!("✓ TUNNELING_ACK received ({} bytes)", len);

    // Handle ACK
    client.handle_tunneling_ack(&buffer[..len]).unwrap();
    println!("✓ TUNNELING_ACK processed");
}

#[test]
#[ignore]
fn test_tunnel_disconnect() {
    println!("\n=== Test: Tunnel Disconnect ===");

    let socket = create_test_socket().expect("Failed to create socket");

    // Connect
    let client = TunnelClient::new(SIMULATOR_IP, SIMULATOR_PORT);
    let (client, connect_frame) = client.connect().unwrap();
    socket.send_to(connect_frame, simulator_addr()).unwrap();

    let mut buffer = [0u8; 1024];
    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    let client = client.handle_connect_response(&buffer[6..len]).unwrap();
    println!("✓ Connected");

    // Disconnect
    let disconnect_frame = client.disconnect().unwrap();
    println!("✓ DISCONNECT_REQUEST built ({} bytes)", disconnect_frame.len());

    socket.send_to(&disconnect_frame, simulator_addr()).unwrap();
    println!("✓ DISCONNECT_REQUEST sent");

    // Receive DISCONNECT_RESPONSE
    let (len, _) = socket.recv_from(&mut buffer).unwrap();
    println!("✓ DISCONNECT_RESPONSE received ({} bytes)", len);
}

#[test]
fn test_group_address_creation() {
    println!("\n=== Test: Group Address Creation ===");

    // 3-level addressing
    let addr = GroupAddress::new(1, 2, 3).expect("Failed to create group address");
    assert_eq!(addr.to_raw(), 0x0A03);
    println!("✓ GroupAddress::new(1, 2, 3) -> 0x{:04X}", addr.to_raw());

    // From raw
    let addr = GroupAddress::from(0x0A03);
    assert_eq!(addr.to_raw(), 0x0A03);
    println!("✓ GroupAddress::from(0x0A03) OK");
}

#[test]
fn test_individual_address_creation() {
    println!("\n=== Test: Individual Address Creation ===");

    let addr = IndividualAddress::new(1, 1, 250).expect("Failed to create individual address");
    assert_eq!(addr.to_raw(), 0x11FA);
    println!("✓ IndividualAddress::new(1, 1, 250) -> 0x{:04X}", addr.to_raw());

    let addr = IndividualAddress::from(0x11FA);
    assert_eq!(addr.to_raw(), 0x11FA);
    println!("✓ IndividualAddress::from(0x11FA) OK");
}

/// Helper function to build a test CEMI frame
fn build_test_cemi_frame() -> [u8; 11] {
    use knx_rs::protocol::cemi::{ControlField1, ControlField2};
    use knx_rs::protocol::constants::CEMIMessageCode;

    let mut frame = [0u8; 11];
    frame[0] = CEMIMessageCode::LDataReq.to_u8();
    frame[1] = 0x00; // No additional info
    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();
    // Source: 1.1.250
    frame[4] = 0x11;
    frame[5] = 0xFA;
    // Destination: 1/2/3
    frame[6] = 0x0A;
    frame[7] = 0x03;
    frame[8] = 0x01; // NPDU length
    frame[9] = 0x00; // TPCI
    frame[10] = 0x81; // APCI + value (ON)

    frame
}
