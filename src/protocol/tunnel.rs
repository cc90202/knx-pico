//! KNXnet/IP Tunneling Client with Typestate Pattern
//!
//! Tunneling client for KNX gateway communication over IP networks.
//!
//! Uses the typestate pattern to enforce correct state transitions at
//! compile-time, preventing invalid operations.
//!
//! ## Features
//!
//! - **Compile-time state validation** using typestate pattern
//! - Connection management (CONNECT/DISCONNECT)
//! - Heartbeat / keep-alive (CONNECTIONSTATE)
//! - Reliable message delivery with ACK
//! - Sequence counter management
//! - Zero runtime overhead for state checks
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::protocol::tunnel::TunnelClient;
//!
//! // Create client (starts in Idle state)
//! let client = TunnelClient::new([192, 168, 1, 10], 3671);
//!
//! // Connect (Idle → Connecting)
//! let (client, connect_frame) = client.connect()?;
//! // send connect_frame over network...
//!
//! // Handle response (Connecting → Connected)
//! let mut client = client.handle_connect_response(&response_data)?;
//!
//! // Send data (only available in Connected state!)
//! let frame = client.send_tunneling_request(&cemi_data)?;
//!
//! // Disconnect (Connected → Disconnecting)
//! let (client, disc_frame) = client.disconnect()?;
//!
//! // Finish (Disconnecting → Idle)
//! let client = client.finish();
//! ```

use crate::error::{KnxError, Result};
use crate::protocol::constants::MAX_FRAME_SIZE;
use crate::protocol::frame::Hpai;
use crate::protocol::services::{
    ConnectRequest, ConnectResponse, ConnectionHeader, ConnectionStateRequest,
    ConnectionStateResponse, DisconnectRequest, TunnelingAck, TunnelingRequest,
};

/// Maximum buffer size for frames
const BUFFER_SIZE: usize = MAX_FRAME_SIZE;

// =============================================================================
// State Types (Zero-Sized Types for compile-time state tracking)
// =============================================================================

/// Client is idle (not connected)
#[derive(Debug, Clone, Copy)]
pub struct Idle;

/// Connection request sent, waiting for response
#[derive(Debug, Clone, Copy)]
pub struct Connecting {
    /// Length of the frame data in `tx_buffer`
    pub frame_len: usize,
}

/// Connected and ready to send/receive
#[derive(Debug, Clone, Copy)]
pub struct Connected {
    /// Communication channel ID assigned by gateway
    pub channel_id: u8,
    /// Send sequence counter (wraps at 256)
    pub send_sequence: u8,
    /// Receive sequence counter (expected)
    pub recv_sequence: u8,
}

/// Disconnect request sent
#[derive(Debug, Clone, Copy)]
pub struct Disconnecting {
    /// Length of the frame data in `tx_buffer`
    pub frame_len: usize,
}

// =============================================================================
// Tunneling Client with Generic State Parameter
// =============================================================================

/// Tunneling client for KNX gateway communication.
///
/// Uses typestate pattern for compile-time state validation.
/// The state parameter `S` determines which methods are available.
///
/// ## State Machine
///
/// ```text
/// Idle → Connecting → Connected → Disconnecting → Idle
///         ↓ (error)     ↓ (error)
///         Idle          Idle
/// ```
///
/// ## Type Safety
///
/// - `TunnelClient<Idle>` can only call `connect()`
/// - `TunnelClient<Connecting>` can only call `handle_connect_response()` or `cancel()`
/// - `TunnelClient<Connected>` can only call send/receive methods or `disconnect()`
/// - `TunnelClient<Disconnecting>` can only call `finish()`
///
/// This prevents invalid operations like sending data when not connected.
pub struct TunnelClient<State> {
    /// Gateway IP address
    gateway_addr: [u8; 4],
    /// Gateway port
    gateway_port: u16,
    /// Local control endpoint
    control_endpoint: Hpai,
    /// Local data endpoint
    data_endpoint: Hpai,
    /// Buffer for building frames
    tx_buffer: [u8; BUFFER_SIZE],
    /// Buffer for receiving frames
    #[allow(dead_code)] // Reserved for future receive buffer optimization
    rx_buffer: [u8; BUFFER_SIZE],
    /// Current state (type changes based on state!)
    state: State,
}

// =============================================================================
// Debug implementation for TunnelClient
// =============================================================================

impl<S: core::fmt::Debug> core::fmt::Debug for TunnelClient<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TunnelClient")
            .field("gateway_addr", &self.gateway_addr)
            .field("gateway_port", &self.gateway_port)
            .field("state", &self.state)
            .finish()
    }
}

// =============================================================================
// Methods Available in ALL States
// =============================================================================

impl<S> TunnelClient<S> {
    /// Get gateway address (available in all states)
    #[inline]
    pub const fn gateway_addr(&self) -> ([u8; 4], u16) {
        (self.gateway_addr, self.gateway_port)
    }
}

// =============================================================================
// Methods Available ONLY in Idle State
// =============================================================================

impl TunnelClient<Idle> {
    /// Create a new tunnel client (starts in Idle state)
    ///
    /// # Arguments
    /// * `gateway_addr` - IP address of the KNX gateway (accepts array, tuple, or `Ipv4Addr`)
    /// * `gateway_port` - Port number (typically 3671)
    ///
    /// # Examples
    /// ```rust,no_run
    /// use knx_pico::protocol::tunnel::TunnelClient;
    /// use knx_pico::Ipv4Addr;
    ///
    /// // From array
    /// let client = TunnelClient::new([192, 168, 1, 10], 3671);
    ///
    /// // From tuple
    /// let client = TunnelClient::new((192, 168, 1, 10), 3671);
    ///
    /// // From Ipv4Addr
    /// let addr = Ipv4Addr::new(192, 168, 1, 10);
    /// let client = TunnelClient::new(addr, 3671);
    /// ```
    pub fn new(gateway_addr: impl Into<crate::net::Ipv4Addr>, gateway_port: u16) -> Self {
        let gateway_addr = gateway_addr.into().octets();
        // Use NAT mode (0.0.0.0:0) - gateway will respond to source address
        let nat_endpoint = Hpai::new([0, 0, 0, 0], 0);

        TunnelClient {
            gateway_addr,
            gateway_port,
            control_endpoint: nat_endpoint,
            data_endpoint: nat_endpoint,
            tx_buffer: [0u8; BUFFER_SIZE],
            rx_buffer: [0u8; BUFFER_SIZE],
            state: Idle,
        }
    }

    /// Create tunnel client with explicit local endpoint (Routing mode)
    ///
    /// Some gateways don't support NAT mode and need real IP.
    ///
    /// # Arguments
    /// * `gateway_addr` - IP address of the KNX gateway (accepts array, tuple, or `Ipv4Addr`)
    /// * `gateway_port` - Port number (typically 3671)
    /// * `local_addr` - Local IP address of this device (accepts array, tuple, or `Ipv4Addr`)
    /// * `local_port` - Local port (typically 3671)
    ///
    /// # Examples
    /// ```rust,no_run
    /// use knx_pico::protocol::tunnel::TunnelClient;
    ///
    /// // From arrays
    /// let client = TunnelClient::new_with_local_endpoint(
    ///     [192, 168, 1, 10],
    ///     3671,
    ///     [192, 168, 1, 100],
    ///     3671
    /// );
    ///
    /// // From tuples
    /// let client = TunnelClient::new_with_local_endpoint(
    ///     (192, 168, 1, 10),
    ///     3671,
    ///     (192, 168, 1, 100),
    ///     3671
    /// );
    /// ```
    pub fn new_with_local_endpoint(
        gateway_addr: impl Into<crate::net::Ipv4Addr>,
        gateway_port: u16,
        local_addr: impl Into<crate::net::Ipv4Addr>,
        local_port: u16,
    ) -> Self {
        let gateway_addr = gateway_addr.into().octets();
        let local_addr = local_addr.into().octets();
        let local_endpoint = Hpai::new(local_addr, local_port);

        TunnelClient {
            gateway_addr,
            gateway_port,
            control_endpoint: local_endpoint,
            data_endpoint: local_endpoint,
            tx_buffer: [0u8; BUFFER_SIZE],
            rx_buffer: [0u8; BUFFER_SIZE],
            state: Idle,
        }
    }

    /// Start connection (Idle → Connecting)
    ///
    /// Consumes self and returns a new client in Connecting state.
    /// The `CONNECT_REQUEST` frame is available in the client's internal buffer.
    ///
    /// # Returns
    /// - New client in Connecting state with frame ready to send
    ///
    /// # Example
    /// ```rust,no_run
    /// let client = TunnelClient::new([192, 168, 1, 10], 3671);
    /// let mut client = client.connect()?;
    /// let frame = client.frame_data();
    /// // send frame over network...
    /// ```
    pub fn connect(mut self) -> Result<TunnelClient<Connecting>> {
        // Build CONNECT_REQUEST
        let request = ConnectRequest::new(self.control_endpoint, self.data_endpoint);
        let len = request.build(&mut self.tx_buffer)?;

        // Transition to Connecting state
        Ok(TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Connecting { frame_len: len },
        })
    }
}

// =============================================================================
// Methods Available ONLY in Connecting State
// =============================================================================

impl TunnelClient<Connecting> {
    /// Get the `CONNECT_REQUEST` frame data to send
    ///
    /// Returns a reference to the frame in the internal buffer.
    /// Valid until the next state transition.
    #[inline]
    pub fn frame_data(&self) -> &[u8] {
        &self.tx_buffer[..self.state.frame_len]
    }

    /// Handle `CONNECT_RESPONSE` (Connecting → Connected)
    ///
    /// Processes the gateway's response to our connection request.
    ///
    /// # Arguments
    /// * `response` - `CONNECT_RESPONSE` frame body
    ///
    /// # Returns
    /// - `Ok(TunnelClient<Connected>)` - Connection successful
    /// - `Err(KnxError)` - Connection failed (client dropped, create new one)
    ///
    /// # Example
    /// ```rust,no_run
    /// let (client, _) = client.connect()?;
    /// // ... receive response_data from network ...
    /// let client = client.handle_connect_response(&response_data)?;
    /// // Now client is in Connected state!
    /// ```
    pub fn handle_connect_response(self, response: &[u8]) -> Result<TunnelClient<Connected>> {
        let resp = ConnectResponse::parse(response)?;

        if !resp.is_ok() {
            // Connection failed - client is dropped, caller should create new one
            return Err(KnxError::connection_failed());
        }

        // Transition to Connected state
        Ok(TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Connected {
                channel_id: resp.channel_id,
                send_sequence: 0,
                recv_sequence: 0,
            },
        })
    }

    /// Cancel connection attempt (Connecting → Idle)
    ///
    /// Returns to Idle state without completing connection.
    pub fn cancel(self) -> TunnelClient<Idle> {
        TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Idle,
        }
    }
}

// =============================================================================
// Methods Available ONLY in Connected State
// =============================================================================

impl TunnelClient<Connected> {
    /// Get assigned channel ID (only available when connected)
    #[inline]
    pub const fn channel_id(&self) -> u8 {
        self.state.channel_id
    }

    /// Get current send sequence counter
    #[inline]
    pub const fn send_sequence(&self) -> u8 {
        self.state.send_sequence
    }

    /// Get current receive sequence counter
    #[inline]
    pub const fn recv_sequence(&self) -> u8 {
        self.state.recv_sequence
    }

    /// Send `TUNNELING_REQUEST`
    ///
    /// No state check needed - if you're here, you're connected!
    ///
    /// # Arguments
    /// * `cemi_data` - cEMI frame data to send
    ///
    /// # Returns
    /// Frame data to send to gateway. Automatically increments send sequence.
    ///
    /// # Example
    /// ```rust,no_run
    /// let cemi_frame = [...]; // cEMI data
    /// let frame = client.send_tunneling_request(&cemi_frame)?;
    /// // send frame over network...
    /// ```
    pub fn send_tunneling_request(&mut self, cemi_data: &[u8]) -> Result<&[u8]> {
        let header = ConnectionHeader::new(self.state.channel_id, self.state.send_sequence);
        let request = TunnelingRequest::new(header, cemi_data);
        let len = request.build(&mut self.tx_buffer)?;

        // Increment sequence counter (wraps at 256)
        self.state.send_sequence = self.state.send_sequence.wrapping_add(1);

        Ok(&self.tx_buffer[..len])
    }

    /// Build `TUNNELING_ACK` frame
    ///
    /// # Arguments
    /// * `sequence` - Sequence number to acknowledge
    /// * `status` - Status code (0 = OK)
    pub fn build_tunneling_ack(&mut self, sequence: u8, status: u8) -> Result<&[u8]> {
        let header = ConnectionHeader::new(self.state.channel_id, sequence);
        let ack = TunnelingAck::new(header, status);
        let len = ack.build(&mut self.tx_buffer)?;
        Ok(&self.tx_buffer[..len])
    }

    /// Handle `TUNNELING_INDICATION` (incoming event from gateway)
    ///
    /// # Arguments
    /// * `body` - `TUNNELING_REQUEST` frame body
    ///
    /// # Returns
    /// cEMI data from the tunnel request
    ///
    /// # Errors
    /// Returns `SequenceMismatch` if sequence counter is wrong
    pub fn handle_tunneling_indication<'a>(&mut self, body: &'a [u8]) -> Result<&'a [u8]> {
        let request = TunnelingRequest::parse(body)?;

        // Verify sequence counter
        if request.connection_header.sequence_counter != self.state.recv_sequence {
            return Err(KnxError::sequence_mismatch());
        }

        // Increment receive sequence counter
        self.state.recv_sequence = self.state.recv_sequence.wrapping_add(1);

        Ok(request.cemi_data)
    }

    /// Handle `TUNNELING_ACK`
    ///
    /// Verifies the ACK status
    pub fn handle_tunneling_ack(&self, body: &[u8]) -> Result<()> {
        let ack = TunnelingAck::parse(body)?;

        if !ack.is_ok() {
            return Err(KnxError::tunneling_ack_failed());
        }

        Ok(())
    }

    /// Send `CONNECTIONSTATE_REQUEST` (heartbeat)
    ///
    /// Used to check if connection is still alive
    pub fn send_heartbeat(&mut self) -> Result<&[u8]> {
        let request = ConnectionStateRequest::new(self.state.channel_id, self.control_endpoint);
        let len = request.build(&mut self.tx_buffer)?;
        Ok(&self.tx_buffer[..len])
    }

    /// Handle `CONNECTIONSTATE_RESPONSE`
    ///
    /// On error, automatically transitions to Idle
    pub fn handle_heartbeat_response(self, body: &[u8]) -> Result<TunnelClient<Connected>> {
        let response = ConnectionStateResponse::parse(body)?;

        if !response.is_ok() {
            // Connection lost
            return Err(KnxError::connection_lost());
        }

        Ok(self)
    }

    /// Start disconnect (Connected → Disconnecting)
    ///
    /// # Returns
    /// - New client in Disconnecting state with frame ready to send
    ///
    /// # Example
    /// ```rust,no_run
    /// let mut client = client.disconnect()?;
    /// let frame = client.frame_data();
    /// // send frame over network...
    /// ```
    pub fn disconnect(mut self) -> Result<TunnelClient<Disconnecting>> {
        let request = DisconnectRequest::new(self.state.channel_id, self.control_endpoint);
        let len = request.build(&mut self.tx_buffer)?;

        // Transition to Disconnecting state
        Ok(TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Disconnecting { frame_len: len },
        })
    }
}

// =============================================================================
// Methods Available ONLY in Disconnecting State
// =============================================================================

impl TunnelClient<Disconnecting> {
    /// Get the `DISCONNECT_REQUEST` frame data to send
    ///
    /// Returns a reference to the frame in the internal buffer.
    /// Valid until the next state transition.
    #[inline]
    pub fn frame_data(&self) -> &[u8] {
        &self.tx_buffer[..self.state.frame_len]
    }

    /// Handle `DISCONNECT_RESPONSE` and finish (Disconnecting → Idle)
    ///
    /// Always returns to Idle state, even if response indicates error.
    ///
    /// # Example
    /// ```rust,no_run
    /// let (client, disc_frame) = client.disconnect()?;
    /// // send disc_frame...
    /// // receive response...
    /// let client = client.finish(&response_data)?;
    /// // Client is back in Idle state
    /// ```
    pub fn finish(self, _response: &[u8]) -> Result<TunnelClient<Idle>> {
        // Parse response (we don't really care about errors at this point)
        // Always transition back to Idle
        Ok(TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Idle,
        })
    }

    /// Finish without waiting for response (emergency disconnect)
    pub fn finish_now(self) -> TunnelClient<Idle> {
        TunnelClient {
            gateway_addr: self.gateway_addr,
            gateway_port: self.gateway_port,
            control_endpoint: self.control_endpoint,
            data_endpoint: self.data_endpoint,
            tx_buffer: self.tx_buffer,
            rx_buffer: self.rx_buffer,
            state: Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::constants::SERVICE_CONNECTIONSTATE_REQUEST;

    #[test]
    fn test_client_creation() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        assert_eq!(client.gateway_addr(), ([192, 168, 1, 10], 3671));
    }

    #[test]
    fn test_state_transitions() {
        // Create client (Idle)
        let client = TunnelClient::new([192, 168, 1, 10], 3671);

        // Connect (Idle → Connecting)
        let client = client.connect().unwrap();
        let connect_frame = client.frame_data();
        assert!(connect_frame.len() >= 26);
        assert_eq!(connect_frame[0], 0x06);

        // Simulate CONNECT_RESPONSE
        let response_data = [
            0x05, 0x00, // Channel=5, Status=OK
            0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, // HPAI
            0x04, 0x04, 0x02, 0x00, // CRD
        ];

        // Handle response (Connecting → Connected)
        let mut client = client.handle_connect_response(&response_data).unwrap();
        assert_eq!(client.channel_id(), 5);
        assert_eq!(client.send_sequence(), 0);
        assert_eq!(client.recv_sequence(), 0);

        // Send data (only possible in Connected state!)
        let cemi_data = &[0x29, 0x00, 0xBC, 0xE0];
        let tunnel_frame = client.send_tunneling_request(cemi_data).unwrap();
        assert!(tunnel_frame.len() > 0);
        assert_eq!(client.send_sequence(), 1); // Incremented

        // Disconnect (Connected → Disconnecting)
        let client = client.disconnect().unwrap();
        let disc_frame = client.frame_data();
        assert!(disc_frame.len() >= 16);

        // Finish (Disconnecting → Idle)
        let client = client.finish(&[0x05, 0x00]).unwrap();

        // Can reconnect!
        let _client = client.connect().unwrap();
    }

    #[test]
    fn test_sequence_wrapping() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x03, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let mut client = client.handle_connect_response(&response).unwrap();

        // Manually set sequence to 255
        client.state.send_sequence = 255;

        // Send should wrap to 0
        let _ = client.send_tunneling_request(&[0x29, 0x00]).unwrap();
        assert_eq!(client.send_sequence(), 0);
    }

    #[test]
    fn test_connect_error() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        // Error response
        let error_response = [
            0x00, 0x24, // No channel, error status
            0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];

        let result = client.handle_connect_response(&error_response);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Connection(_)));
    }

    #[test]
    fn test_cancel_connection() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        // Cancel and go back to Idle
        let client = client.cancel();

        // Can connect again
        let _client = client.connect().unwrap();
    }

    #[test]
    fn test_tunneling_indication() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x03, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let mut client = client.handle_connect_response(&response).unwrap();

        // Receive indication
        let indication_data = [
            0x04, 0x03, 0x00, 0x00, // Header: len=4, channel=3, seq=0, reserved
            0x29, 0x00, 0xBC, 0xE0, // cEMI data
        ];

        let cemi = client
            .handle_tunneling_indication(&indication_data)
            .unwrap();
        assert_eq!(cemi, &[0x29, 0x00, 0xBC, 0xE0]);
        assert_eq!(client.recv_sequence(), 1);
    }

    #[test]
    fn test_sequence_mismatch() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x03, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let mut client = client.handle_connect_response(&response).unwrap();

        // Set recv_sequence to 5
        client.state.recv_sequence = 5;

        // Receive indication with wrong sequence
        let indication_data = [
            0x04, 0x03, 0x03, 0x00, // Sequence = 3 (expected 5)
            0x29, 0x00,
        ];

        let result = client.handle_tunneling_indication(&indication_data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Tunneling(_)));
    }

    #[test]
    fn test_heartbeat() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x03, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let mut client = client.handle_connect_response(&response).unwrap();

        // Send heartbeat
        let heartbeat_frame = client.send_heartbeat().unwrap();
        assert!(heartbeat_frame.len() >= 16);
        assert_eq!(
            u16::from_be_bytes([heartbeat_frame[2], heartbeat_frame[3]]),
            SERVICE_CONNECTIONSTATE_REQUEST
        );

        // Handle response
        let hb_response = [0x03, 0x00]; // Channel, Status OK
        let client = client.handle_heartbeat_response(&hb_response).unwrap();

        // Still connected
        assert_eq!(client.channel_id(), 3);
    }

    #[test]
    fn test_heartbeat_failure() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x03, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let client = client.handle_connect_response(&response).unwrap();

        // Failed heartbeat
        let hb_response = [0x03, 0x26]; // Error status
        let result = client.handle_heartbeat_response(&hb_response);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Connection(_)));
    }

    #[test]
    fn test_tunneling_ack() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x05, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let mut client = client.handle_connect_response(&response).unwrap();

        // Build ACK
        let ack_frame = client.build_tunneling_ack(10, 0).unwrap();
        assert!(ack_frame.len() >= 11);

        // Handle ACK
        let ack_data = [
            0x04, 0x05, 0x00, 0x00, // Header
            0x00, // Status OK
        ];
        let result = client.handle_tunneling_ack(&ack_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_finish_now() {
        let client = TunnelClient::new([192, 168, 1, 10], 3671);
        let client = client.connect().unwrap();

        let response = [
            0x03, 0x00, 0x08, 0x01, 192, 168, 1, 10, 0x0E, 0x57, 0x04, 0x04, 0x02, 0x00,
        ];
        let client = client.handle_connect_response(&response).unwrap();

        // Emergency disconnect
        let client = client.disconnect().unwrap();
        let client = client.finish_now();

        // Back to Idle
        let _client = client.connect().unwrap();
    }
}
