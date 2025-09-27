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

/// Timeout for connection establishment
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for receiving responses
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);

/// Recommended heartbeat interval (KNX spec: 60 seconds)
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum UDP packet size for KNXnet/IP
const MAX_PACKET_SIZE: usize = 512;

/// Async wrapper for TunnelClient with embassy-net UDP
///
/// # Note on Memory Usage
/// This struct contains two 512-byte buffers (rx_buffer, cemi_buffer).
/// The socket uses separate buffers passed to `new()`.
/// Total memory: ~1KB for this struct + socket buffers.
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
    /// Internal tunnel client (when connected)
    client: Option<TunnelClient<Connected>>,
}

impl<'a> AsyncTunnelClient<'a> {
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
        // Create tunnel client
        let tunnel = TunnelClient::new(self.gateway_addr, self.gateway_port);

        // Build CONNECT_REQUEST
        let (tunnel, frame_data) = tunnel.connect()?;

        // Bind to any local port
        self.socket.bind(0).map_err(|_| KnxError::SocketError)?;

        // Send CONNECT_REQUEST
        let gateway = self.gateway_endpoint();

        self.socket
            .send_to(frame_data, gateway)
            .await
            .map_err(|_| KnxError::SocketError)?;

        // Wait for CONNECT_RESPONSE with timeout
        let (n, _remote) = with_timeout(CONNECT_TIMEOUT, self.socket.recv_from(&mut self.rx_buffer))
            .await
            .map_err(|_| KnxError::Timeout)?
            .map_err(|_| KnxError::SocketError)?;

        // Parse frame
        let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

        // Verify service type
        if frame.service_type() != ServiceType::ConnectResponse {
            return Err(KnxError::InvalidFrame);
        }

        // Handle CONNECT_RESPONSE
        let connected_client = tunnel.handle_connect_response(frame.body())?;

        self.client = Some(connected_client);

        Ok(())
    }

    /// Send cEMI frame over tunnel
    ///
    /// # Arguments
    /// * `cemi_data` - cEMI frame data to send
    ///
    /// # Returns
    /// - `Ok(())` - Frame sent successfully
    /// - `Err(KnxError)` - Send failed
    ///
    /// # Example
    /// ```rust,no_run
    /// client.send_cemi(&cemi_frame).await?;
    /// ```
    pub async fn send_cemi(&mut self, cemi_data: &[u8]) -> Result<()> {
        // Get gateway endpoint before borrowing client
        let gateway = self.gateway_endpoint();

        let client = self.client.as_mut().ok_or(KnxError::NotConnected)?;

        // Build TUNNELING_REQUEST
        let frame_data = client.send_tunneling_request(cemi_data)?;

        // Send via UDP
        self.socket
            .send_to(frame_data, gateway)
            .await
            .map_err(|_| KnxError::SocketError)?;

        // Wait for ACK
        let (n, _) = with_timeout(RESPONSE_TIMEOUT, self.socket.recv_from(&mut self.rx_buffer))
            .await
            .map_err(|_| KnxError::Timeout)?
            .map_err(|_| KnxError::SocketError)?;

        let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

        if frame.service_type() == ServiceType::TunnellingAck {
            client.handle_tunneling_ack(frame.body())?;
        }

        Ok(())
    }

    /// Receive cEMI frame from gateway (non-blocking with timeout)
    ///
    /// # Returns
    /// - `Ok(Some(data))` - Received cEMI frame
    /// - `Ok(None)` - No data available (timeout)
    /// - `Err(KnxError)` - Receive error
    ///
    /// # Example
    /// ```rust,no_run
    /// if let Some(cemi) = client.receive().await? {
    ///     // Process cEMI frame
    /// }
    /// ```
    pub async fn receive(&mut self) -> Result<Option<&[u8]>> {
        // Get gateway endpoint before borrowing client
        let gateway = self.gateway_endpoint();

        let client = self.client.as_mut().ok_or(KnxError::NotConnected)?;

        // Try to receive with timeout
        let result = with_timeout(
            Duration::from_millis(100),
            self.socket.recv_from(&mut self.rx_buffer)
        ).await;

        match result {
            Ok(Ok((n, _))) => {
                let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

                match frame.service_type() {
                    ServiceType::TunnellingRequest => {
                        // Handle TUNNELING_INDICATION
                        let cemi_data = client.handle_tunneling_indication(frame.body())?;

                        // Send ACK (handle_tunneling_indication already incremented recv_sequence)
                        // We need to ACK the sequence we just received
                        let ack_seq = client.recv_sequence().wrapping_sub(1);
                        let ack_frame = client.build_tunneling_ack(ack_seq, 0)?;

                        self.socket
                            .send_to(ack_frame, gateway)
                            .await
                            .map_err(|_| KnxError::SocketError)?;

                        // Copy cEMI data to our buffer to avoid lifetime issues
                        let len = cemi_data.len();
                        self.cemi_buffer[..len].copy_from_slice(cemi_data);

                        Ok(Some(&self.cemi_buffer[..len]))
                    }
                    _ => Ok(None),
                }
            }
            Ok(Err(_)) => Err(KnxError::SocketError),
            Err(_) => Ok(None), // Timeout - no data
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
            let (_, frame_data) = client.disconnect()?;

            // Send via UDP
            let gateway = self.gateway_endpoint();

            self.socket
                .send_to(frame_data, gateway)
                .await
                .map_err(|_| KnxError::SocketError)?;

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

        let client = self.client.as_mut().ok_or(KnxError::NotConnected)?;

        // Build CONNECTIONSTATE_REQUEST
        let heartbeat_frame = client.send_heartbeat()?;

        // Send via UDP
        self.socket
            .send_to(heartbeat_frame, gateway)
            .await
            .map_err(|_| KnxError::SocketError)?;

        // Wait for CONNECTIONSTATE_RESPONSE
        let (n, _) = with_timeout(RESPONSE_TIMEOUT, self.socket.recv_from(&mut self.rx_buffer))
            .await
            .map_err(|_| KnxError::Timeout)?
            .map_err(|_| KnxError::SocketError)?;

        let frame = KnxnetIpFrame::parse(&self.rx_buffer[..n])?;

        if frame.service_type() == ServiceType::ConnectionstateResponse {
            // Handle response and check if connection is still alive
            let connected_client = self.client.take().ok_or(KnxError::NotConnected)?;
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
