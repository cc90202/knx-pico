#![cfg(not(test))]

//! KNX Gateway Discovery via SEARCH_REQUEST
//!
//! This module implements the KNXnet/IP SEARCH protocol to automatically
//! discover KNX gateways on the local network. This eliminates the need
//! for hardcoded gateway IP addresses in configuration.
//!
//! ## Protocol Flow
//!
//! ```text
//! Client                          Gateway
//!   |                                |
//!   |------- SEARCH_REQUEST -------->| (multicast)
//!   |<------ SEARCH_RESPONSE --------|
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::knx_discovery::discover_gateway;
//!
//! // Discover first available gateway (3 second timeout)
//! if let Some(gateway) = discover_gateway(&stack, Duration::from_secs(3)).await {
//!     info!("Found gateway at {}:{}", gateway.ip, gateway.port);
//! }
//! ```

use embassy_net::{udp::{UdpSocket, PacketMetadata}, IpEndpoint, Stack};
use embassy_time::{Duration, Timer, with_timeout};

/// Discovered KNX gateway information
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GatewayInfo {
    /// Gateway IP address as [u8; 4]
    pub ip: [u8; 4],
    /// Gateway port (typically 3671)
    pub port: u16,
}

/// KNX multicast address for discovery (224.0.23.12)
const KNX_MULTICAST_ADDR: [u8; 4] = [224, 0, 23, 12];

/// Standard KNX port
const KNX_PORT: u16 = 3671;

/// Build a SEARCH_REQUEST packet
///
/// Format:
/// ```text
/// Header (6 bytes):
///   0x06          - header_len
///   0x10          - protocol_version
///   0x02 0x01     - service_type (SEARCH_REQUEST)
///   0x00 0x0e     - total_length (14 bytes)
///
/// HPAI (8 bytes):
///   0x08          - structure_length
///   0x01          - protocol_code (IPv4 UDP)
///   [4 bytes]     - local IP address
///   [2 bytes]     - local port (big-endian)
/// ```
fn build_search_request(local_ip: [u8; 4], local_port: u16, buffer: &mut [u8]) -> usize {
    // Header
    buffer[0] = 0x06;  // header_len (CRITICAL: this was the missing byte!)
    buffer[1] = 0x10;  // protocol_version
    buffer[2] = 0x02;  // service_type high byte
    buffer[3] = 0x01;  // service_type low byte (SEARCH_REQUEST)
    buffer[4] = 0x00;  // total_length high byte
    buffer[5] = 0x0e;  // total_length low byte (14 bytes)

    // HPAI (Host Protocol Address Information)
    buffer[6] = 0x08;  // structure_length
    buffer[7] = 0x01;  // protocol_code (UDP)
    buffer[8..12].copy_from_slice(&local_ip);
    buffer[12..14].copy_from_slice(&local_port.to_be_bytes());

    14  // Total packet size
}

/// Parse a SEARCH_RESPONSE packet
///
/// Format:
/// ```text
/// Header (6 bytes) - standard KNXnet/IP header
/// HPAI (8 bytes) - gateway's control endpoint
/// DIB blocks (variable) - device information (optional, ignored for now)
/// ```
fn parse_search_response(data: &[u8]) -> Option<GatewayInfo> {
    // Minimum valid response: header (6) + HPAI (8) = 14 bytes
    if data.len() < 14 {
        return None;
    }

    // Verify header
    if data[0] != 0x06 || data[1] != 0x10 {
        return None;
    }

    // Check service type is SEARCH_RESPONSE (0x0202)
    if data[2] != 0x02 || data[3] != 0x02 {
        return None;
    }

    // Parse HPAI starting at byte 6
    let hpai_len = data[6];
    let protocol = data[7];

    if hpai_len != 0x08 || protocol != 0x01 {
        return None;
    }

    // Extract IP and port
    let ip = [data[8], data[9], data[10], data[11]];
    let port = u16::from_be_bytes([data[12], data[13]]);

    Some(GatewayInfo { ip, port })
}

/// Discover KNX gateway on the local network
///
/// Sends SEARCH_REQUEST to multicast address and waits for first response.
///
/// # Arguments
///
/// * `stack` - Network stack reference
/// * `timeout` - Maximum time to wait for response
///
/// # Returns
///
/// Returns `Some(GatewayInfo)` if a gateway is found, `None` otherwise.
///
/// # Example
///
/// ```rust,no_run
/// let gateway = discover_gateway(&stack, Duration::from_secs(3)).await;
/// ```
pub async fn discover_gateway(
    stack: &Stack<'static>,
    timeout: Duration,
) -> Option<GatewayInfo> {
    // Get local IP address
    let local_config = stack.config_v4()?;
    let local_ip = local_config.address.address().octets();

    // Create UDP socket with metadata buffers
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buffer = [0u8; 512];
    let mut tx_buffer = [0u8; 512];
    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    // Bind to any available port
    if socket.bind(0).is_err() {
        return None;
    }

    let local_endpoint = socket.endpoint();
    let local_port = local_endpoint.port;

    // Build SEARCH_REQUEST
    let mut request_buf = [0u8; 14];
    let request_len = build_search_request(local_ip, local_port, &mut request_buf);

    // Send to multicast address
    let [a, b, c, d] = KNX_MULTICAST_ADDR;
    let multicast_endpoint = IpEndpoint::new(
        embassy_net::Ipv4Address::new(a, b, c, d).into(),
        KNX_PORT,
    );

    // Try to send (ignore errors, UDP is best-effort)
    let _ = socket.send_to(&request_buf[..request_len], multicast_endpoint).await;

    // Also try broadcast to local subnet
    if let Some(_gateway_addr) = local_config.gateway {
        let broadcast_addr = calculate_broadcast(local_ip, local_config.address.prefix_len());
        let [a, b, c, d] = broadcast_addr;
        let broadcast_endpoint = IpEndpoint::new(
            embassy_net::Ipv4Address::new(a, b, c, d).into(),
            KNX_PORT,
        );
        let _ = socket.send_to(&request_buf[..request_len], broadcast_endpoint).await;
    }

    // Wait for response with timeout
    let mut response_buf = [0u8; 256];

    match with_timeout(timeout, async {
        loop {
            if let Ok((len, _remote)) = socket.recv_from(&mut response_buf).await {
                if let Some(gateway) = parse_search_response(&response_buf[..len]) {
                    return Some(gateway);
                }
            }
            Timer::after_millis(10).await;
        }
    }).await {
        Ok(gateway) => gateway,
        Err(_) => None,  // Timeout
    }
}

/// Calculate broadcast address for a given IP and prefix length
fn calculate_broadcast(ip: [u8; 4], prefix_len: u8) -> [u8; 4] {
    if prefix_len >= 32 {
        return ip;
    }

    let host_bits = 32 - prefix_len;
    let mask = !((1u32 << host_bits) - 1);

    let ip_u32 = u32::from_be_bytes(ip);
    let broadcast_u32 = ip_u32 | !mask;

    broadcast_u32.to_be_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_request() {
        let mut buf = [0u8; 14];
        let len = build_search_request([192, 168, 1, 29], 12345, &mut buf);

        assert_eq!(len, 14);
        assert_eq!(buf[0], 0x06);  // header_len
        assert_eq!(buf[1], 0x10);  // protocol_version
        assert_eq!(buf[2], 0x02);  // service_type high
        assert_eq!(buf[3], 0x01);  // service_type low (SEARCH_REQUEST)
        assert_eq!(buf[4], 0x00);  // total_length high
        assert_eq!(buf[5], 0x0e);  // total_length low (14)
        assert_eq!(buf[6], 0x08);  // HPAI length
        assert_eq!(buf[7], 0x01);  // protocol (UDP)
        assert_eq!(&buf[8..12], &[192, 168, 1, 29]);  // IP
        assert_eq!(u16::from_be_bytes([buf[12], buf[13]]), 12345);  // port
    }

    #[test]
    fn test_parse_search_response() {
        let response = [
            0x06, 0x10,         // header_len, protocol_version
            0x02, 0x02,         // service_type (SEARCH_RESPONSE)
            0x00, 0x0e,         // total_length
            0x08, 0x01,         // HPAI: length, protocol
            192, 168, 1, 250,   // IP address
            0x0e, 0x57,         // port (3671 in big-endian)
        ];

        let gateway = parse_search_response(&response).unwrap();
        assert_eq!(gateway.ip, [192, 168, 1, 250]);
        assert_eq!(gateway.port, 3671);
    }

    #[test]
    fn test_calculate_broadcast() {
        // /24 network
        assert_eq!(
            calculate_broadcast([192, 168, 1, 29], 24),
            [192, 168, 1, 255]
        );

        // /16 network
        assert_eq!(
            calculate_broadcast([10, 0, 5, 10], 16),
            [10, 0, 255, 255]
        );
    }
}

