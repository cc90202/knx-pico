//! KNXnet/IP Tunneling Client
//!
//! This module provides a tunneling client for communicating with KNX gateways
//! over IP networks. The client handles connection management, heartbeat, and
//! reliable message delivery.
//!
//! ## Features
//!
//! - Connection management (CONNECT/DISCONNECT)
//! - Heartbeat / keep-alive (CONNECTIONSTATE)
//! - Reliable message delivery with ACK
//! - Sequence counter management
//! - State machine for connection lifecycle
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_rs::protocol::tunnel::TunnelClient;
//!
//! let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
//! client.connect()?;
//! client.send_cemi(&cemi_frame)?;
//! let response = client.receive()?;
//! client.disconnect()?;
//! ```

use crate::error::{KnxError, Result};
use crate::protocol::constants::*;
use crate::protocol::frame::{Hpai, KnxnetIpFrame};
use crate::protocol::services::*;

/// Maximum buffer size for frames
const BUFFER_SIZE: usize = MAX_FRAME_SIZE;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Idle,
    /// Connection request sent, waiting for response
    Connecting,
    /// Connected and ready
    Connected,
    /// Disconnect request sent
    Disconnecting,
}

/// Tunneling client for KNX gateway communication
///
/// This client manages a stateful connection to a KNXnet/IP gateway,
/// handling all protocol details including sequence counters, heartbeat,
/// and acknowledgments.
///
/// ## State Machine
///
/// ```text
/// Idle → Connecting → Connected → Disconnecting → Idle
///         ↓ (error)     ↓ (timeout)
///         Idle          Idle
/// ```
pub struct TunnelClient {
    /// Gateway IP address
    gateway_addr: [u8; 4],
    /// Gateway port
    gateway_port: u16,
    /// Current connection state
    state: ConnectionState,
    /// Communication channel ID (assigned by gateway)
    channel_id: u8,
    /// Send sequence counter (wraps at 256)
    send_sequence: u8,
    /// Receive sequence counter (expected)
    recv_sequence: u8,
    /// Local control endpoint
    control_endpoint: Hpai,
    /// Local data endpoint
    data_endpoint: Hpai,
    /// Buffer for building frames
    tx_buffer: [u8; BUFFER_SIZE],
    /// Buffer for receiving frames
    #[allow(dead_code)]
    rx_buffer: [u8; BUFFER_SIZE],
}

impl TunnelClient {
    /// Create a new tunnel client
    ///
    /// # Arguments
    /// * `gateway_addr` - IP address of the KNX gateway
    /// * `gateway_port` - Port number (typically 3671)
    ///
    /// # Example
    /// ```rust,no_run
    /// let client = TunnelClient::new([192, 168, 1, 10], 3671);
    /// ```
    pub fn new(gateway_addr: [u8; 4], gateway_port: u16) -> Self {
        // Use NAT mode (0.0.0.0:0) - gateway will respond to source address
        let nat_endpoint = Hpai::new([0, 0, 0, 0], 0);

        Self {
            gateway_addr,
            gateway_port,
            state: ConnectionState::Idle,
            channel_id: 0,
            send_sequence: 0,
            recv_sequence: 0,
            control_endpoint: nat_endpoint,
            data_endpoint: nat_endpoint,
            tx_buffer: [0u8; BUFFER_SIZE],
            rx_buffer: [0u8; BUFFER_SIZE],
        }
    }

    /// Get current connection state
    #[inline]
    pub const fn state(&self) -> ConnectionState {
        self.state
    }

    /// Check if connected
    #[inline]
    pub const fn is_connected(&self) -> bool {
        matches!(self.state, ConnectionState::Connected)
    }

    /// Get assigned channel ID
    #[inline]
    pub const fn channel_id(&self) -> u8 {
        self.channel_id
    }

    /// Get gateway address
    #[inline]
    pub const fn gateway_addr(&self) -> ([u8; 4], u16) {
        (self.gateway_addr, self.gateway_port)
    }

    /// Build a CONNECT_REQUEST frame
    ///
    /// Returns the frame data and length
    fn build_connect_request(&mut self) -> Result<&[u8]> {
        let request = ConnectRequest::new(self.control_endpoint, self.data_endpoint);
        let len = request.build(&mut self.tx_buffer)?;
        Ok(&self.tx_buffer[..len])
    }

    /// Build a CONNECTIONSTATE_REQUEST frame
    fn build_connectionstate_request(&mut self) -> Result<&[u8]> {
        let request = ConnectionStateRequest::new(self.channel_id, self.control_endpoint);
        let len = request.build(&mut self.tx_buffer)?;
        Ok(&self.tx_buffer[..len])
    }

    /// Build a DISCONNECT_REQUEST frame
    fn build_disconnect_request(&mut self) -> Result<&[u8]> {
        let request = DisconnectRequest::new(self.channel_id, self.control_endpoint);
        let len = request.build(&mut self.tx_buffer)?;
        Ok(&self.tx_buffer[..len])
    }

    /// Build a TUNNELING_REQUEST frame
    ///
    /// # Arguments
    /// * `cemi_data` - cEMI frame data to send
    ///
    /// # Returns
    /// The frame data and increments send sequence counter
    fn build_tunneling_request(&mut self, cemi_data: &[u8]) -> Result<&[u8]> {
        let header = ConnectionHeader::new(self.channel_id, self.send_sequence);
        let request = TunnelingRequest::new(header, cemi_data);
        let len = request.build(&mut self.tx_buffer)?;

        // Increment sequence counter (wraps at 256)
        self.send_sequence = self.send_sequence.wrapping_add(1);

        Ok(&self.tx_buffer[..len])
    }

    /// Build a TUNNELING_ACK frame
    ///
    /// # Arguments
    /// * `sequence` - Sequence number to acknowledge
    /// * `status` - Status code (0 = OK)
    fn build_tunneling_ack(&mut self, sequence: u8, status: u8) -> Result<&[u8]> {
        let header = ConnectionHeader::new(self.channel_id, sequence);
        let ack = TunnelingAck::new(header, status);
        let len = ack.build(&mut self.tx_buffer)?;
        Ok(&self.tx_buffer[..len])
    }

    /// Parse a received frame
    ///
    /// # Arguments
    /// * `data` - Frame data received from network
    ///
    /// # Returns
    /// Parsed frame or error
    #[allow(dead_code)]
    fn parse_frame<'a>(&self, data: &'a [u8]) -> Result<KnxnetIpFrame<'a>> {
        KnxnetIpFrame::parse(data)
    }

    /// Handle CONNECT_RESPONSE
    ///
    /// Updates state and stores channel ID if successful
    fn handle_connect_response(&mut self, body: &[u8]) -> Result<()> {
        let response = ConnectResponse::parse(body)?;

        if !response.is_ok() {
            self.state = ConnectionState::Idle;
            return Err(KnxError::ConnectionFailed);
        }

        self.channel_id = response.channel_id;
        self.send_sequence = 0;
        self.recv_sequence = 0;
        self.state = ConnectionState::Connected;

        Ok(())
    }

    /// Handle CONNECTIONSTATE_RESPONSE
    #[allow(dead_code)]
    fn handle_connectionstate_response(&mut self, body: &[u8]) -> Result<()> {
        let response = ConnectionStateResponse::parse(body)?;

        if !response.is_ok() {
            // Connection lost or error
            self.state = ConnectionState::Idle;
            return Err(KnxError::ConnectionLost);
        }

        Ok(())
    }

    /// Handle DISCONNECT_RESPONSE
    fn handle_disconnect_response(&mut self, body: &[u8]) -> Result<()> {
        let _response = DisconnectResponse::parse(body)?;

        // Always go to idle, even if response indicates error
        self.state = ConnectionState::Idle;
        self.channel_id = 0;
        self.send_sequence = 0;
        self.recv_sequence = 0;

        Ok(())
    }

    /// Handle TUNNELING_REQUEST (indication from gateway)
    ///
    /// Returns the cEMI data if valid
    fn handle_tunneling_request<'a>(&mut self, body: &'a [u8]) -> Result<&'a [u8]> {
        let request = TunnelingRequest::parse(body)?;

        // Verify sequence counter
        if request.connection_header.sequence_counter != self.recv_sequence {
            // Sequence mismatch - potential packet loss
            return Err(KnxError::SequenceMismatch);
        }

        // Increment receive sequence counter
        self.recv_sequence = self.recv_sequence.wrapping_add(1);

        // Return cEMI data for processing
        Ok(request.cemi_data)
    }

    /// Handle TUNNELING_ACK
    ///
    /// Verifies the ACK matches our last sent sequence
    fn handle_tunneling_ack(&self, body: &[u8]) -> Result<()> {
        let ack = TunnelingAck::parse(body)?;

        if !ack.is_ok() {
            return Err(KnxError::TunnelingAckFailed);
        }

        // Note: Sequence verification should be done by caller tracking sent requests
        // Here we just verify the ACK status

        Ok(())
    }

    /// Reset connection state
    ///
    /// Useful for error recovery
    pub fn reset(&mut self) {
        self.state = ConnectionState::Idle;
        self.channel_id = 0;
        self.send_sequence = 0;
        self.recv_sequence = 0;
    }
}

// Note: Actual network I/O will be added in Phase 4 (Embassy integration)
// For now, this provides the protocol state machine and frame building

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        assert_eq!(client.state(), ConnectionState::Idle);
        assert_eq!(client.channel_id(), 0);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_gateway_addr() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        assert_eq!(client.gateway_addr(), ([192, 168, 1, 10], 3671));
    }

    #[test]
    fn test_build_connect_request() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        let frame = client.build_connect_request().unwrap();

        assert!(frame.len() >= 26); // Minimum CONNECT_REQUEST size
        assert_eq!(frame[0], 0x06); // Header length
        assert_eq!(frame[1], 0x10); // Protocol version
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), SERVICE_CONNECT_REQUEST);
    }

    #[test]
    fn test_build_disconnect_request() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 5; // Simulate connected state

        let frame = client.build_disconnect_request().unwrap();

        assert!(frame.len() >= 16);
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), SERVICE_DISCONNECT_REQUEST);
    }

    #[test]
    fn test_build_connectionstate_request() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 5;

        let frame = client.build_connectionstate_request().unwrap();

        assert!(frame.len() >= 16);
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), SERVICE_CONNECTIONSTATE_REQUEST);
    }

    #[test]
    fn test_build_tunneling_request() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 3;
        client.send_sequence = 0;

        let cemi_data = &[0x29, 0x00, 0xBC, 0xE0];
        let frame = client.build_tunneling_request(cemi_data).unwrap();

        assert!(frame.len() >= 10 + cemi_data.len());
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), SERVICE_TUNNELING_REQUEST);

        // Sequence should increment
        assert_eq!(client.send_sequence, 1);
    }

    #[test]
    fn test_sequence_wrapping() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 3;
        client.send_sequence = 255;

        let cemi_data = &[0x29, 0x00];
        client.build_tunneling_request(cemi_data).unwrap();

        // Should wrap to 0
        assert_eq!(client.send_sequence, 0);
    }

    #[test]
    fn test_build_tunneling_ack() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 5;

        let frame = client.build_tunneling_ack(10, 0).unwrap();

        assert!(frame.len() >= 11);
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), SERVICE_TUNNELING_ACK);
    }

    #[test]
    fn test_handle_connect_response_success() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.state = ConnectionState::Connecting;

        // Minimal valid CONNECT_RESPONSE
        let response_data = [
            0x05, // Channel ID
            0x00, // Status = OK
            // HPAI (8 bytes)
            0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57,
            // CRD (4 bytes)
            0x04, 0x04, 0x02, 0x00,
        ];

        client.handle_connect_response(&response_data).unwrap();

        assert_eq!(client.state(), ConnectionState::Connected);
        assert_eq!(client.channel_id(), 5);
        assert_eq!(client.send_sequence, 0);
        assert_eq!(client.recv_sequence, 0);
    }

    #[test]
    fn test_handle_connect_response_error() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.state = ConnectionState::Connecting;

        // CONNECT_RESPONSE with error status
        let response_data = [
            0x00, // Channel ID (not assigned)
            0x24, // Status = E_NO_MORE_CONNECTIONS
            0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57,
            0x04, 0x04, 0x02, 0x00,
        ];

        let result = client.handle_connect_response(&response_data);

        assert!(result.is_err());
        assert_eq!(client.state(), ConnectionState::Idle);
    }

    #[test]
    fn test_handle_disconnect_response() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.state = ConnectionState::Disconnecting;
        client.channel_id = 5;

        let response_data = [0x05, 0x00]; // Channel ID, Status

        client.handle_disconnect_response(&response_data).unwrap();

        assert_eq!(client.state(), ConnectionState::Idle);
        assert_eq!(client.channel_id(), 0);
    }

    #[test]
    fn test_handle_tunneling_request() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 3;
        client.recv_sequence = 0;

        // TUNNELING_REQUEST body
        let request_data = [
            // Connection header (4 bytes)
            0x04, // Structure length
            0x03, // Channel ID
            0x00, // Sequence = 0
            0x00, // Reserved
            // cEMI data
            0x29, 0x00, 0xBC, 0xE0,
        ];

        let cemi_data = client.handle_tunneling_request(&request_data).unwrap();

        assert_eq!(cemi_data, &[0x29, 0x00, 0xBC, 0xE0]);
        assert_eq!(client.recv_sequence, 1);
    }

    #[test]
    fn test_handle_tunneling_request_sequence_mismatch() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 3;
        client.recv_sequence = 5; // Expecting sequence 5

        // Request with sequence 3
        let request_data = [
            0x04, 0x03, 0x03, 0x00, // Wrong sequence
            0x29, 0x00,
        ];

        let result = client.handle_tunneling_request(&request_data);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::SequenceMismatch));
    }

    #[test]
    fn test_handle_tunneling_ack() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);

        // TUNNELING_ACK body
        let ack_data = [
            0x04, // Structure length
            0x03, // Channel ID
            0x00, // Sequence
            0x00, // Reserved
            0x00, // Status = OK
        ];

        let result = client.handle_tunneling_ack(&ack_data);

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_tunneling_ack_error() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);

        // TUNNELING_ACK with error status
        let ack_data = [
            0x04, 0x03, 0x00, 0x00,
            0x29, // Status = error
        ];

        let result = client.handle_tunneling_ack(&ack_data);

        assert!(result.is_err());
    }

    #[test]
    fn test_reset() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.state = ConnectionState::Connected;
        client.channel_id = 5;
        client.send_sequence = 10;
        client.recv_sequence = 8;

        client.reset();

        assert_eq!(client.state(), ConnectionState::Idle);
        assert_eq!(client.channel_id(), 0);
        assert_eq!(client.send_sequence, 0);
        assert_eq!(client.recv_sequence, 0);
    }

    #[test]
    fn test_recv_sequence_wrapping() {
        let mut client = TunnelClient::new([192, 168, 1, 10], 3671);
        client.channel_id = 3;
        client.recv_sequence = 255;

        let request_data = [
            0x04, 0x03, 0xFF, 0x00, // Sequence = 255
            0x29, 0x00,
        ];

        client.handle_tunneling_request(&request_data).unwrap();

        // Should wrap to 0
        assert_eq!(client.recv_sequence, 0);
    }
}
