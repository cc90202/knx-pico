//! Async KNXnet/IP Tunneling Client for Embassy
//!
//! This module provides an async wrapper around the TunnelClient that integrates
//! with embassy-net UDP sockets for real network communication.
//!
//! ## Features
//!
//! - Full async/await support with Embassy
//! - UDP socket integration with embassy-net
//! - Automatic connection management
//! - Heartbeat/keep-alive (call `send_heartbeat()` every 60s)
//! - Timeout handling
//! - Clean error handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_rs::protocol::async_tunnel::AsyncTunnelClient;
//! use embassy_net::Stack;
//!
//! let mut client = AsyncTunnelClient::new(
//!     stack,
//!     [192, 168, 1, 10],
//!     3671,
//! );
//!
//! // Connect to gateway
//! client.connect().await?;
//!
//! // Send command
//! client.send_cemi(&cemi_frame).await?;
//!
//! // Receive events
//! if let Some(cemi) = client.receive().await? {
//!     // Process received data
//! }
//!
//! // Disconnect
//! client.disconnect().await?;
//! ```

use crate::error::{KnxError, Result};
use crate::protocol::tunnel::{TunnelClient, Connected};
use crate::protocol::frame::KnxnetIpFrame;
use crate::protocol::constants::ServiceType;
use embassy_net::{Stack, udp::{UdpSocket, PacketMetadata}};
use embassy_time::{Duration, with_timeout};

// Import unified logging macro from crate root
use crate::pico_log;

/// Timeout for connection establishment
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for receiving responses
///
/// **CRITICAL**: Set to 200ms to prevent system crashes on Pico 2 W.
/// KNX gateways typically respond within 50-100ms, so 200ms is sufficient.
/// Longer timeouts (500ms+) cause stack overflow and hard resets on embedded devices.
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(200);

/// Recommended heartbeat interval (KNX spec: 60 seconds)
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum UDP packet size for KNXnet/IP
const MAX_PACKET_SIZE: usize = 512;

// =============================================================================
// Production Configuration Constants
// =============================================================================
// These values have been tuned for real-world KNX installations.
// Adjust them based on your specific installation characteristics:
//
// - Small home (5-20 devices): Use default values
// - Medium building (20-100 devices): Increase FLUSH_TIMEOUT to 100ms
// - Large installation (100+ devices): Increase FLUSH_TIMEOUT to 150ms
//                                       and MAX_FLUSH_PACKETS to 50
// =============================================================================

/// Timeout for flushing pending packets before sending new command
///
/// **Critical for production reliability**
///
/// This timeout determines how long we wait for pending bus events before
/// sending a new command. In busy installations, increase this value.
///
/// Recommended values:
/// - Small home (5-20 devices): 600ms (optimized for fast response)
/// - Medium building (20-100 devices): 1000ms
/// - Large installation (100+ devices): 1500ms
///
/// **Production value: 600ms**
/// This value has been empirically determined through hardware testing with Pico 2 W.
/// KNX gateways typically send TUNNELING_INDICATION ~500ms after ACK. The 600ms
/// timeout provides a 100ms safety margin for network jitter while maintaining
/// fast response times.
///
/// Note: This is a timeout, not a fixed delay. If no packets are pending, the flush
/// exits immediately. The 600ms value balances speed and reliability.
const FLUSH_TIMEOUT: Duration = Duration::from_millis(600);

/// Maximum number of packets to flush before sending a new command
///
/// **Safety limit to prevent infinite loops**
///
/// This prevents the flush loop from running forever if the bus is
/// extremely busy or malfunctioning. If this limit is hit, a warning
/// is logged and the command is sent anyway.
///
/// Recommended values:
/// - Normal installations: 20 (default)
/// - Very busy installations: 50
const MAX_FLUSH_PACKETS: usize = 20;

/// Maximum number of TUNNELING_INDICATION messages to process while waiting for ACK
///
/// **Prevents timeout on busy bus and system crashes**
///
/// After sending a command, we wait for ACK. During this time, the gateway
/// may send TUNNELING_INDICATION for other bus events. This limits how many
/// we process before considering the command successful.
///
/// **IMPORTANT**: On embedded devices like Pico 2 W, high values can cause
/// stack overflow and crashes. Value of 1 is safest for production:
/// - First INDICATION = echo of our command (gateway confirmation)
/// - Command is immediately considered successful after first INDICATION
///
/// Many KNX gateways don't send separate ACK, only echo the command as INDICATION.
/// After receiving and ACKing the INDICATION, we consider the command successful.
const MAX_ACK_WAIT_INDICATIONS: usize = 1;

/// Maximum number of recv_from attempts to prevent system crash
///
/// Limits the total number of recv_from() calls to prevent stack overflow.
/// On Pico 2 W, multiple recv_from() calls with timeouts can crash the system.
const MAX_RECV_ATTEMPTS: usize = 2;

/// Async wrapper for TunnelClient with embassy-net UDP
///
/// # Note on Memory Usage
/// This struct contains three 512-byte buffers (rx_buffer, cemi_buffer, pending_indication_buffer).
/// The socket uses separate buffers passed to `new()`.
/// Total memory: ~1.5KB for this struct + socket buffers.
pub struct AsyncTunnelClient<'a> {
    /// UDP socket for communication
    socket: UdpSocket<'a>,
    /// Gateway address
    gateway_addr: [u8; 4],
    /// Gateway port
    gateway_port: u16,
    /// Receive buffer for UDP packets
    rx_buffer: [u8; MAX_PACKET_SIZE],
    /// Temporary buffer for cEMI data copying (to avoid lifetime issues)
    cemi_buffer: [u8; MAX_PACKET_SIZE],
    /// Buffer for pending INDICATION received during send_cemi()
    pending_indication_buffer: [u8; MAX_PACKET_SIZE],
    /// Length of pending INDICATION (None if no pending data)
    pending_indication_len: Option<usize>,
    /// Internal tunnel client (when connected)
    client: Option<TunnelClient<Connected>>,
}

impl<'a> core::fmt::Debug for AsyncTunnelClient<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncTunnelClient")
            .field("gateway_addr", &self.gateway_addr)
            .field("gateway_port", &self.gateway_port)
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl<'a> AsyncTunnelClient<'a> {
    /// Safe buffer copy with bounds checking
    ///
    /// Prevents buffer overflow panics by checking sizes before copying.
    /// Returns error if source data is too large for destination buffer.
    fn safe_buffer_copy(dest: &mut [u8], src: &[u8]) -> Result<usize> {
        if src.len() > dest.len() {
            pico_log!(error, "Buffer overflow prevented: src_len={} > dest_len={}", src.len(), dest.len());
            return Err(KnxError::invalid_frame());
        }
        dest[..src.len()].copy_from_slice(src);
        Ok(src.len())
    }

    /// Create a new async tunnel client
    ///
    /// # Arguments
    /// * `stack` - Embassy network stack
    /// * `gateway_addr` - IP address of KNX gateway
    /// * `gateway_port` - Port of KNX gateway (typically 3671)
    ///
    /// # Example
    /// ```rust,no_run
    /// let client = AsyncTunnelClient::new(stack, [192, 168, 1, 10], 3671);
    /// ```
    pub fn new(
        stack: &'a Stack<'a>,
        rx_meta: &'a mut [PacketMetadata],
        tx_meta: &'a mut [PacketMetadata],
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
        gateway_addr: [u8; 4],
        gateway_port: u16,
    ) -> Self {
        let socket = UdpSocket::new(*stack, rx_meta, rx_buffer, tx_meta, tx_buffer);

        Self {
            socket,
            pending_indication_buffer: [0u8; MAX_PACKET_SIZE],
            pending_indication_len: None,
            gateway_addr,
            gateway_port,
            rx_buffer: [0u8; MAX_PACKET_SIZE],
            cemi_buffer: [0u8; MAX_PACKET_SIZE],
            client: None,
        }
    }

    /// Helper to create gateway endpoint (DRY principle)
    fn gateway_endpoint(&self) -> embassy_net::IpEndpoint {
        embassy_net::IpEndpoint::new(
            embassy_net::IpAddress::v4(
                self.gateway_addr[0],
                self.gateway_addr[1],
                self.gateway_addr[2],
                self.gateway_addr[3],
            ),
            self.gateway_port,
        )
    }

    /// Connect to KNX gateway
    ///
    /// Sends CONNECT_REQUEST and waits for CONNECT_RESPONSE.
    ///
    /// # Returns
    /// - `Ok(())` - Connection successful
    /// - `Err(KnxError)` - Connection failed
    ///
    /// # Example
    /// ```rust,no_run
    /// client.connect().await?;
    /// ```
    pub async fn connect(&mut self) -> Result<()> {
        // Bind socket first to get local port
        self.socket.bind(0).map_err(|_| KnxError::socket_error())?;

        // Get local IP address for Routing mode
        // Some KNX gateways don't support NAT mode (0.0.0.0:0) and require the real IP
        let ep = self.socket.endpoint();
        let (local_ip, local_port) = if let Some(embassy_net::IpAddress::Ipv4(ipv4)) = ep.addr {
            (ipv4.octets(), ep.port)
        } else {
            ([0, 0, 0, 0], 0) // Fallback to NAT mode
        };

        pico_log!(info, "Local endpoint: {}.{}.{}.{}:{}",
            local_ip[0], local_ip[1], local_ip[2], local_ip[3], local_port);

        // Create tunnel client with Routing mode (use real IP and port)
        let tunnel = TunnelClient::new_with_local_endpoint(
            self.gateway_addr,
            self.gateway_port,
            local_ip,
            local_port,
        );

        // Build CONNECT_REQUEST
        let tunnel = tunnel.connect()?;
        let frame_data = tunnel.frame_data();

        // Log the CONNECT_REQUEST details
        pico_log!(debug, "CONNECT_REQUEST frame ({} bytes):", frame_data.len());
        pico_log!(debug, "  Header: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
            frame_data[0], frame_data[1], frame_data[2], frame_data[3], frame_data[4], frame_data[5]);
        if frame_data.len() >= 26 {
            pico_log!(debug, "  Control endpoint: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                frame_data[6], frame_data[7], frame_data[8], frame_data[9],
                frame_data[10], frame_data[11], frame_data[12], frame_data[13]);
            pico_log!(debug, "  Data endpoint: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                frame_data[14], frame_data[15], frame_data[16], frame_data[17],
                frame_data[18], frame_data[19], frame_data[20], frame_data[21]);
            pico_log!(debug, "  CRI: {:02x} {:02x} {:02x} {:02x}",
                frame_data[22], frame_data[23], frame_data[24], frame_data[25]);
        }

        // Send CONNECT_REQUEST
        let gateway = self.gateway_endpoint();
        pico_log!(info, "Sending CONNECT_REQUEST to {}.{}.{}.{}:{}",
            self.gateway_addr[0], self.gateway_addr[1], self.gateway_addr[2], self.gateway_addr[3],
            self.gateway_port);

        self.socket
            .send_to(frame_data, gateway)
            .await
            .map_err(|_| KnxError::socket_error())?;

        pico_log!(info, "CONNECT_REQUEST sent, waiting for response (timeout: {}s)...", CONNECT_TIMEOUT.as_secs());

        // Wait for CONNECT_RESPONSE with timeout
        let (n, _remote) = with_timeout(CONNECT_TIMEOUT, self.socket.recv_from(&mut self.rx_buffer))
            .await
            .map_err(|_| {
                pico_log!(warn, "Timeout waiting for CONNECT_RESPONSE after {}s", CONNECT_TIMEOUT.as_secs());
                KnxError::Timeout
            })?
            .map_err(|_| KnxError::socket_error())?;

        // Extract IPv4 address directly (KNX only uses IPv4, pattern always matches)
        let embassy_net::IpAddress::Ipv4(ipv4) = _remote.endpoint.addr;
        let remote_ip = ipv4.octets();
        pico_log!(info, "Received {} bytes from {}.{}.{}.{}:{}", n, remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3], _remote.endpoint.port);

        // Parse frame
        let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

        // Verify service type
        if frame.service_type() != ServiceType::ConnectResponse {
            return Err(KnxError::invalid_frame());
        }

        // Handle CONNECT_RESPONSE
        let connected_client = tunnel.handle_connect_response(frame.body())?;

        self.client = Some(connected_client);

        Ok(())
    }

    /// Send cEMI frame over tunnel (Fire-and-Forget)
    ///
    /// **Production-Ready Embedded Approach**
    ///
    /// This method sends the command and returns immediately without waiting
    /// for ACK/INDICATION. This "fire-and-forget" approach is standard practice
    /// for embedded systems with limited resources and prevents system crashes.
    ///
    /// # Rationale
    ///
    /// 1. **Crash Prevention**: Waiting for ACK causes stack overflow on Pico 2 W
    /// 2. **Performance**: No 200ms+ wait per command
    /// 3. **Reliability**: Works even if gateway doesn't respond
    /// 4. **Standard Practice**: Common pattern in embedded KNX implementations
    ///
    /// # Verification
    ///
    /// To verify command delivery, use `receive()` to monitor bus events.
    /// The gateway will echo your command as a TUNNELING_INDICATION.
    ///
    /// # Arguments
    /// * `cemi_data` - cEMI frame data to send
    ///
    /// # Returns
    /// - `Ok(())` - Frame sent to network (UDP layer confirms delivery)
    /// - `Err(KnxError)` - Network send failed
    ///
    /// # Example
    /// ```rust,no_run
    /// // Send command
    /// client.send_cemi(&cemi_frame).await?;
    ///
    /// // Optional: verify delivery by monitoring bus events
    /// if let Some(cemi) = client.receive().await? {
    ///     // Check if it's our command echo
    /// }
    /// ```
    pub async fn send_cemi(&mut self, cemi_data: &[u8]) -> Result<()> {
        // Validate input size
        if cemi_data.len() > MAX_PACKET_SIZE {
            pico_log!(error, "cEMI too large: {}", cemi_data.len());
            return Err(KnxError::invalid_frame());
        }

        pico_log!(info, "send_cemi: len={}", cemi_data.len());

        // Get gateway endpoint before borrowing client
        let gateway = self.gateway_endpoint();

        let client = self.client.as_mut().ok_or_else(|| {
            pico_log!(error, "Not connected!");
            KnxError::not_connected()
        })?;

        pico_log!(info, "Flush start");

        // Flush any pending packets (e.g., late TUNNELING_INDICATION from previous commands)
        // This is critical for production: the KNX bus can send events at any time
        let mut flushed_count = 0;
        loop {
            // Safety limit: prevent infinite loop on extremely busy bus
            if flushed_count >= MAX_FLUSH_PACKETS {
                pico_log!(warn, "Reached max flush limit ({} packets), bus may be extremely busy", MAX_FLUSH_PACKETS);
                break;
            }

            let result = with_timeout(
                FLUSH_TIMEOUT,
                self.socket.recv_from(&mut self.rx_buffer)
            ).await;

            match result {
                Ok(Ok((n, _))) => {
                    flushed_count += 1;

                    // Process pending packet - use safe parsing
                    if n > MAX_PACKET_SIZE {
                        pico_log!(error, "Packet too large: {}", n);
                        continue;
                    }

                    if let Ok(frame) = KnxnetIpFrame::parse(&self.rx_buffer[..n]) {
                        if frame.service_type() == ServiceType::TunnellingRequest {
                            // ACK the pending TUNNELING_INDICATION
                            if let Ok(_cemi_data) = client.handle_tunneling_indication(frame.body()) {
                                let ack_seq = client.recv_sequence().wrapping_sub(1);
                                if let Ok(ack_frame) = client.build_tunneling_ack(ack_seq, 0) {
                                    let _ = self.socket.send_to(ack_frame, gateway).await;
                                }
                                pico_log!(debug, "Flushed TUNNELING_INDICATION #{}, cemi_len={}", flushed_count, _cemi_data.len());
                            }
                        } else {
                            pico_log!(debug, "Flushed non-INDICATION packet #{}", flushed_count);
                        }
                    }
                }
                _ => break, // No more pending packets (timeout or error)
            }
        }

        if flushed_count > 0 {
            pico_log!(info, "Flushed {} pending packets before sending new command", flushed_count);
        } else {
            pico_log!(info, "No pending packets flushed (buffer was clean)");
        }

        pico_log!(info, "Building TUNNELING_REQUEST...");

        // Build TUNNELING_REQUEST
        let frame_data = client.send_tunneling_request(cemi_data)?;

        pico_log!(info, "Sending {} bytes to gateway...", frame_data.len());

        // Send via UDP
        self.socket
            .send_to(frame_data, gateway)
            .await
            .map_err(|_| {
                pico_log!(error, "Socket send failed!");
                KnxError::socket_error()
            })?;

        pico_log!(info, "✓ Command sent successfully (fire-and-forget)");

        // FIRE-AND-FORGET: Return immediately without waiting for ACK
        // This prevents system crashes on embedded devices with limited resources.
        // The UDP layer confirms the packet was sent to the network.
        // To verify delivery, use receive() to monitor gateway responses.

        Ok(())
    }

    /// Receive cEMI frame from gateway (non-blocking with timeout)
    ///
    /// **CRITICAL: Safe for embedded systems**
    ///
    /// This method uses a single recv_from() call with timeout to prevent
    /// stack overflow on devices like Pico 2 W. All errors are caught and
    /// converted to Ok(None) to prevent crashes.
    ///
    /// # Returns
    /// - `Ok(Some(&[u8]))` - cEMI frame data
    /// - `Ok(None)` - No data available (timeout or error)
    ///
    /// # Example
    /// ```rust,no_run
    /// if let Some(cemi) = client.receive().await? {
    ///     // Process cEMI frame
    /// }
    /// ```
    pub async fn receive(&mut self) -> Result<Option<&[u8]>> {
        // First, check if we have a pending INDICATION saved from send_cemi()
        if let Some(len) = self.pending_indication_len.take() {
            pico_log!(debug, "receive: returning pending INDICATION ({} bytes)", len);

            // Copy to cemi_buffer and return
            self.cemi_buffer[..len].copy_from_slice(&self.pending_indication_buffer[..len]);
            return Ok(Some(&self.cemi_buffer[..len]));
        }

        // No pending INDICATION, try to receive new data
        // Get gateway endpoint before borrowing client
        let gateway = self.gateway_endpoint();

        let client = self.client.as_mut().ok_or_else(|| {
            pico_log!(error, "receive: not connected");
            KnxError::not_connected()
        })?;

        // Try to receive with timeout
        // Note: 200ms timeout to allow for network latency and processing
        let result = with_timeout(
            Duration::from_millis(200),
            self.socket.recv_from(&mut self.rx_buffer)
        ).await;

        match result {
            Ok(Ok((n, _))) => {
                // Parse frame with error handling
                let frame = match KnxnetIpFrame::parse(&self.rx_buffer[..n]) {
                    Ok(f) => f,
                    Err(_e) => {
                        pico_log!(warn, "receive: failed to parse frame");
                        return Ok(None);
                    }
                };

                match frame.service_type() {
                    ServiceType::TunnellingRequest => {
                        // Handle TUNNELING_INDICATION with error handling
                        let cemi_data = match client.handle_tunneling_indication(frame.body()) {
                            Ok(data) => data,
                            Err(_e) => {
                                pico_log!(warn, "receive: failed to handle INDICATION");
                                return Ok(None);
                            }
                        };

                        // Send ACK (best effort, don't fail on error)
                        let ack_seq = client.recv_sequence().wrapping_sub(1);
                        if let Ok(ack_frame) = client.build_tunneling_ack(ack_seq, 0) {
                            let _ = self.socket.send_to(ack_frame, gateway).await;
                        }

                        // Copy cEMI data to our buffer to avoid lifetime issues
                        let len = cemi_data.len();

                        pico_log!(debug, "receive: received INDICATION ({} bytes)", len);

                        self.cemi_buffer[..len].copy_from_slice(cemi_data);

                        Ok(Some(&self.cemi_buffer[..len]))
                    }
                    _ => {
                        pico_log!(debug, "receive: ignored non-INDICATION packet");
                        Ok(None)
                    }
                }
            }
            Ok(Err(_)) => {
                pico_log!(warn, "receive: socket error");
                Ok(None) // Don't fail, just return no data
            }
            Err(_) => {
                pico_log!(trace, "receive: timeout (no data)");
                Ok(None)
            }
        }
    }

    /// Disconnect from gateway
    ///
    /// Sends DISCONNECT_REQUEST and waits for response.
    ///
    /// # Example
    /// ```rust,no_run
    /// client.disconnect().await?;
    /// ```
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            // Build DISCONNECT_REQUEST
            let disconnecting_client = client.disconnect()?;
            let frame_data = disconnecting_client.frame_data();

            // Send via UDP
            let gateway = self.gateway_endpoint();

            self.socket
                .send_to(frame_data, gateway)
                .await
                .map_err(|_| KnxError::socket_error())?;

            // Wait for DISCONNECT_RESPONSE (best effort)
            let _ = with_timeout(
                RESPONSE_TIMEOUT,
                self.socket.recv_from(&mut self.rx_buffer)
            ).await;
        }

        self.socket.close();

        Ok(())
    }

    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Get gateway address
    pub fn gateway_addr(&self) -> ([u8; 4], u16) {
        (self.gateway_addr, self.gateway_port)
    }

    /// Send heartbeat/keep-alive (CONNECTIONSTATE_REQUEST)
    ///
    /// Should be called every 60 seconds to keep connection alive.
    /// The gateway will close the connection if no heartbeat is received.
    ///
    /// # Returns
    /// - `Ok(())` - Heartbeat successful
    /// - `Err(KnxError)` - Heartbeat failed
    ///
    /// # Example
    /// ```rust,no_run
    /// use embassy_time::{Timer, Duration};
    ///
    /// loop {
    ///     Timer::after(Duration::from_secs(60)).await;
    ///     client.send_heartbeat().await?;
    /// }
    /// ```
    pub async fn send_heartbeat(&mut self) -> Result<()> {
        let gateway = self.gateway_endpoint();

        let client = self.client.as_mut().ok_or_else(|| KnxError::not_connected())?;

        // Build CONNECTIONSTATE_REQUEST
        let heartbeat_frame = client.send_heartbeat()?;

        // Send via UDP
        self.socket
            .send_to(heartbeat_frame, gateway)
            .await
            .map_err(|_| KnxError::socket_error())?;

        // Wait for CONNECTIONSTATE_RESPONSE
        let (n, _) = with_timeout(RESPONSE_TIMEOUT, self.socket.recv_from(&mut self.rx_buffer))
            .await
            .map_err(|_| KnxError::Timeout)?
            .map_err(|_| KnxError::socket_error())?;

        let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

        if frame.service_type() == ServiceType::ConnectionstateResponse {
            // Handle response and check if connection is still alive
            let connected_client = self.client.take().ok_or_else(|| KnxError::not_connected())?;
            let connected_client = connected_client.handle_heartbeat_response(frame.body())?;
            self.client = Some(connected_client);
        }

        Ok(())
    }
}

impl<'a> Drop for AsyncTunnelClient<'a> {
    fn drop(&mut self) {
        self.socket.close();
    }
}
