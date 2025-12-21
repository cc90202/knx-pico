//! Async KNXnet/IP Tunneling Client with pluggable transport
//!
//! This module provides an async wrapper around the TunnelClient with
//! support for any transport implementing the `AsyncTransport` trait.
//!
//! ## Features
//!
//! - Full async/await support
//! - Pluggable transport layer (UDP, mock, etc.)
//! - Automatic connection management
//! - Heartbeat/keep-alive (call `send_heartbeat()` every 60s)
//! - Timeout handling
//! - Clean error handling
//! - Dependency Inversion Principle (depends on transport abstraction)
//!
//! ## Example with Embassy UDP
//!
//! ```rust,no_run
//! use knx_pico::protocol::async_tunnel::AsyncTunnelClient;
//! use knx_pico::net::embassy_adapter::EmbassyUdpTransport;
//!
//! let transport = EmbassyUdpTransport::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
//! let mut client = AsyncTunnelClient::new(transport, [192, 168, 1, 10], 3671);
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
//!
//! ## Example with Mock Transport (Testing)
//!
//! ```rust
//! use knx_pico::net::mock_transport::MockTransport;
//! use knx_pico::protocol::async_tunnel::AsyncTunnelClient;
//!
//! let mut mock = MockTransport::new();
//! mock.add_response(vec![0x06, 0x10, 0x02, 0x06, ...]);  // CONNECT_RESPONSE
//!
//! let mut client = AsyncTunnelClient::new(mock, [192, 168, 1, 10], 3671);
//! client.connect().await?;
//! ```

use crate::error::{KnxError, Result};
use crate::net::transport::AsyncTransport;
use crate::net::IpEndpoint;
use crate::protocol::constants::{ServiceType, MAX_CEMI_SIZE};
use crate::protocol::frame::KnxnetIpFrame;
use crate::protocol::tunnel::{Connected, TunnelClient};
use embassy_time::{with_timeout, Duration};
use heapless::{Deque, Vec as HVec};

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

/// Maximum pending INDICATION messages in queue
///
/// When sending multiple commands in sequence, INDICATION responses are
/// queued here during the flush loop. The queue uses a circular buffer
/// strategy: if full, the oldest message is dropped.
///
/// Memory usage: MAX_PENDING_INDICATIONS * MAX_CEMI_SIZE = 10 * 64 = 640 bytes
const MAX_PENDING_INDICATIONS: usize = 10;

/// Async wrapper for TunnelClient with pluggable transport
///
/// # Type Parameters
///
/// - `T` - Transport implementation (e.g., `EmbassyUdpTransport`, `MockTransport`)
///
/// # Design Pattern
///
/// This struct follows the **Dependency Inversion Principle**:
/// - Depends on `AsyncTransport` trait (abstraction)
/// - Not tied to any specific transport implementation
/// - Enables testing with mock transports
/// - Supports alternative transports (serial, USB, etc.)
///
/// # Note on Memory Usage
///
/// This struct contains two 512-byte buffers (rx_buffer, cemi_buffer) plus a
/// circular queue for pending INDICATION messages (640 bytes).
/// Total memory: ~1.7KB for this struct + transport-specific buffers.
pub struct AsyncTunnelClient<T: AsyncTransport> {
    /// Pluggable transport layer
    transport: T,
    /// Gateway endpoint (IP + port)
    gateway_endpoint: IpEndpoint,
    /// Receive buffer for UDP packets
    rx_buffer: [u8; MAX_PACKET_SIZE],
    /// Temporary buffer for cEMI data copying (to avoid lifetime issues)
    cemi_buffer: [u8; MAX_PACKET_SIZE],
    /// Circular queue for pending INDICATION messages received during send_cemi()
    indication_queue: Deque<HVec<u8, MAX_CEMI_SIZE>, MAX_PENDING_INDICATIONS>,
    /// Internal tunnel client (when connected)
    client: Option<TunnelClient<Connected>>,
}

impl<T: AsyncTransport> core::fmt::Debug for AsyncTunnelClient<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncTunnelClient")
            .field("gateway_endpoint", &self.gateway_endpoint)
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl<T: AsyncTransport> AsyncTunnelClient<T> {
    /// Create a new async tunnel client with the given transport
    ///
    /// # Arguments
    ///
    /// * `transport` - Transport implementation (UDP, mock, etc.)
    /// * `gateway_addr` - IP address of KNX gateway
    /// * `gateway_port` - Port of KNX gateway (typically 3671)
    ///
    /// # Example with Embassy UDP
    ///
    /// ```rust,no_run
    /// use knx_pico::net::embassy_adapter::EmbassyUdpTransport;
    ///
    /// let transport = EmbassyUdpTransport::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
    /// let client = AsyncTunnelClient::new(transport, [192, 168, 1, 10], 3671);
    /// ```
    ///
    /// # Example with Mock (Testing)
    ///
    /// ```rust
    /// use knx_pico::net::mock_transport::MockTransport;
    ///
    /// let transport = MockTransport::new();
    /// let client = AsyncTunnelClient::new(transport, [192, 168, 1, 10], 3671);
    /// ```
    pub fn new(
        transport: T,
        gateway_addr: impl Into<crate::net::Ipv4Addr>,
        gateway_port: u16,
    ) -> Self {
        Self {
            transport,
            gateway_endpoint: IpEndpoint::new(gateway_addr.into(), gateway_port),
            indication_queue: Deque::new(),
            rx_buffer: [0u8; MAX_PACKET_SIZE],
            cemi_buffer: [0u8; MAX_PACKET_SIZE],
            client: None,
        }
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
        // Bind socket to any available port (NAT mode)
        self.transport.bind(0)?;

        // For now, use NAT mode (0.0.0.0:0) which works with most gateways
        // In a real implementation with EmbassyUdpTransport, the adapter would
        // handle binding and return the actual local endpoint
        let (local_ip, local_port) = ([0, 0, 0, 0], 0);

        pico_log!(
            info,
            "Local endpoint: {}.{}.{}.{}:{}",
            local_ip[0],
            local_ip[1],
            local_ip[2],
            local_ip[3],
            local_port
        );

        // Create tunnel client with NAT mode
        let gateway_addr = self.gateway_endpoint.addr.octets();
        let gateway_port = self.gateway_endpoint.port;

        let tunnel =
            TunnelClient::new_with_local_endpoint(gateway_addr, gateway_port, local_ip, local_port);

        // Build CONNECT_REQUEST
        let tunnel = tunnel.connect()?;
        let frame_data = tunnel.frame_data();

        // Log the CONNECT_REQUEST details
        pico_log!(debug, "CONNECT_REQUEST frame ({} bytes):", frame_data.len());
        pico_log!(
            debug,
            "  Header: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
            frame_data[0],
            frame_data[1],
            frame_data[2],
            frame_data[3],
            frame_data[4],
            frame_data[5]
        );
        if frame_data.len() >= 26 {
            pico_log!(
                debug,
                "  Control endpoint: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                frame_data[6],
                frame_data[7],
                frame_data[8],
                frame_data[9],
                frame_data[10],
                frame_data[11],
                frame_data[12],
                frame_data[13]
            );
            pico_log!(
                debug,
                "  Data endpoint: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                frame_data[14],
                frame_data[15],
                frame_data[16],
                frame_data[17],
                frame_data[18],
                frame_data[19],
                frame_data[20],
                frame_data[21]
            );
            pico_log!(
                debug,
                "  CRI: {:02x} {:02x} {:02x} {:02x}",
                frame_data[22],
                frame_data[23],
                frame_data[24],
                frame_data[25]
            );
        }

        // Send CONNECT_REQUEST using transport abstraction
        pico_log!(info, "Sending CONNECT_REQUEST to {}", self.gateway_endpoint);

        self.transport
            .send_to(frame_data, self.gateway_endpoint)
            .await?;

        pico_log!(
            info,
            "CONNECT_REQUEST sent, waiting for response (timeout: {}s)...",
            CONNECT_TIMEOUT.as_secs()
        );

        // Wait for CONNECT_RESPONSE with timeout using transport abstraction
        let (n, remote) = with_timeout(
            CONNECT_TIMEOUT,
            self.transport.recv_from(&mut self.rx_buffer),
        )
        .await
        .map_err(|_| {
            pico_log!(
                warn,
                "Timeout waiting for CONNECT_RESPONSE after {}s",
                CONNECT_TIMEOUT.as_secs()
            );
            KnxError::Timeout
        })??;

        pico_log!(info, "Received {} bytes from {}", n, remote);

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
        let gateway = self.gateway_endpoint;

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
                pico_log!(
                    warn,
                    "Reached max flush limit ({} packets), bus may be extremely busy",
                    MAX_FLUSH_PACKETS
                );
                break;
            }

            let result =
                with_timeout(FLUSH_TIMEOUT, self.transport.recv_from(&mut self.rx_buffer)).await;

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
                            if let Ok(cemi_data) = client.handle_tunneling_indication(frame.body())
                            {
                                let ack_seq = client.recv_sequence().wrapping_sub(1);
                                if let Ok(ack_frame) = client.build_tunneling_ack(ack_seq, 0) {
                                    let _ = self.transport.send_to(ack_frame, gateway).await;
                                }

                                // Queue the INDICATION for later retrieval via receive()
                                let mut vec = HVec::new();
                                if vec.extend_from_slice(cemi_data).is_ok() {
                                    if self.indication_queue.push_back(vec).is_err() {
                                        // Queue full - drop oldest and retry
                                        pico_log!(warn, "Indication queue full, dropping oldest");
                                        self.indication_queue.pop_front();
                                        let mut vec2 = HVec::new();
                                        if vec2.extend_from_slice(cemi_data).is_ok() {
                                            let _ = self.indication_queue.push_back(vec2);
                                        }
                                    }
                                }

                                pico_log!(
                                    debug,
                                    "Queued TUNNELING_INDICATION #{}, cemi_len={}",
                                    flushed_count,
                                    cemi_data.len()
                                );
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
            pico_log!(
                info,
                "Flushed {} pending packets before sending new command",
                flushed_count
            );
        } else {
            pico_log!(info, "No pending packets flushed (buffer was clean)");
        }

        pico_log!(info, "Building TUNNELING_REQUEST...");

        // Build TUNNELING_REQUEST
        let frame_data = client.send_tunneling_request(cemi_data)?;

        pico_log!(info, "Sending {} bytes to gateway...", frame_data.len());

        // Send via transport abstraction
        self.transport.send_to(frame_data, gateway).await?;

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
        // First, check if we have queued INDICATION from send_cemi() flush loop
        if let Some(cemi_vec) = self.indication_queue.pop_front() {
            let len = cemi_vec.len();
            pico_log!(
                debug,
                "receive: returning queued INDICATION ({} bytes)",
                len
            );

            // Copy to cemi_buffer and return
            self.cemi_buffer[..len].copy_from_slice(&cemi_vec);
            return Ok(Some(&self.cemi_buffer[..len]));
        }

        // No queued INDICATION, try to receive new data from socket
        // Get gateway endpoint before borrowing client
        let gateway = self.gateway_endpoint;

        let client = self.client.as_mut().ok_or_else(|| {
            pico_log!(error, "receive: not connected");
            KnxError::not_connected()
        })?;

        // Try to receive with timeout
        let result = with_timeout(
            RESPONSE_TIMEOUT,
            self.transport.recv_from(&mut self.rx_buffer),
        )
        .await;

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

                if frame.service_type() == ServiceType::TunnellingRequest {
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
                        let _ = self.transport.send_to(ack_frame, gateway).await;
                    }

                    // Copy cEMI data to our buffer to avoid lifetime issues
                    let len = cemi_data.len();

                    pico_log!(debug, "receive: received INDICATION ({} bytes)", len);

                    self.cemi_buffer[..len].copy_from_slice(cemi_data);

                    Ok(Some(&self.cemi_buffer[..len]))
                } else {
                    pico_log!(debug, "receive: ignored non-INDICATION packet");
                    Ok(None)
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

            // Send via transport
            self.transport
                .send_to(frame_data, self.gateway_endpoint)
                .await?;

            // Wait for DISCONNECT_RESPONSE (best effort)
            let _ = with_timeout(
                RESPONSE_TIMEOUT,
                self.transport.recv_from(&mut self.rx_buffer),
            )
            .await;
        }

        self.transport.close();

        Ok(())
    }

    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Get gateway endpoint
    pub fn gateway_endpoint(&self) -> IpEndpoint {
        self.gateway_endpoint
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
        let client = self.client.as_mut().ok_or_else(KnxError::not_connected)?;

        // Build CONNECTIONSTATE_REQUEST
        let heartbeat_frame = client.send_heartbeat()?;

        // Send via transport
        self.transport
            .send_to(heartbeat_frame, self.gateway_endpoint)
            .await?;

        // Wait for CONNECTIONSTATE_RESPONSE
        let (n, _) = with_timeout(
            RESPONSE_TIMEOUT,
            self.transport.recv_from(&mut self.rx_buffer),
        )
        .await
        .map_err(|_| KnxError::Timeout)??;

        let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

        if frame.service_type() == ServiceType::ConnectionstateResponse {
            // Handle response and check if connection is still alive
            let connected_client = self.client.take().ok_or_else(KnxError::not_connected)?;
            let connected_client = connected_client.handle_heartbeat_response(frame.body())?;
            self.client = Some(connected_client);
        }

        Ok(())
    }
}

impl<T: AsyncTransport> Drop for AsyncTunnelClient<T> {
    fn drop(&mut self) {
        self.transport.close();
    }
}
