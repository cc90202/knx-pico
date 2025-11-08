//! Network transport abstraction for KNXnet/IP communication.
//!
//! This module provides the `AsyncTransport` trait that abstracts the underlying
//! network transport mechanism, enabling:
//! - Testability through mock implementations
//! - Flexibility to support different transport types (UDP, serial, USB, etc.)
//! - Dependency Inversion Principle compliance
//!
//! ## Design Pattern
//!
//! This follows the **Dependency Inversion Principle (DIP)**:
//! - High-level modules (`AsyncTunnelClient`) depend on abstractions (`AsyncTransport`)
//! - Low-level modules (UDP socket implementations) also depend on the same abstraction
//! - Both can vary independently
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::net::transport::AsyncTransport;
//! use knx_pico::net::IpEndpoint;
//!
//! // Production: use real UDP socket
//! let transport = EmbassyUdpTransport::new(stack);
//! let client = AsyncTunnelClient::new(transport, gateway_addr);
//!
//! // Testing: use mock transport
//! let mut mock = MockTransport::new();
//! mock.add_response(vec![0x06, 0x10, ...]);
//! let client = AsyncTunnelClient::new(mock, gateway_addr);
//! ```

use crate::error::Result;
use crate::net::IpEndpoint;

/// Asynchronous network transport abstraction.
///
/// This trait defines the interface for any network transport mechanism
/// that can send and receive UDP-like datagrams. Implementations can be
/// real network sockets, mock objects for testing, or alternative transports.
///
/// # Design Notes
///
/// The trait is kept minimal to support embedded constraints:
/// - No heap allocations in trait methods
/// - Async/await compatible
/// - Works with `no_std` + `alloc` if needed
///
/// # Examples
///
/// ## Implementing for a custom transport
///
/// ```rust,no_run
/// use knx_pico::net::transport::AsyncTransport;
///
/// struct SerialTransport {
///     // ... serial port fields
/// }
///
/// impl AsyncTransport for SerialTransport {
///     async fn send_to(&mut self, data: &[u8], addr: IpEndpoint) -> Result<()> {
///         // Send data over serial with framing
///         Ok(())
///     }
///
///     async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, IpEndpoint)> {
///         // Receive data from serial
///         Ok((len, addr))
///     }
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait AsyncTransport {
    /// Bind the transport to a local port.
    ///
    /// # Arguments
    ///
    /// * `port` - Local port to bind to (0 = any available port)
    ///
    /// # Errors
    ///
    /// Returns error if the port is already in use or binding fails.
    ///
    /// # Default Implementation
    ///
    /// Default implementation does nothing (no-op). Override if your transport
    /// requires explicit binding before sending/receiving.
    fn bind(&mut self, _port: u16) -> Result<()> {
        Ok(())
    }

    /// Send data to a specific network endpoint.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to send (typically a KNXnet/IP frame)
    /// * `addr` - The destination endpoint (IP + port)
    ///
    /// # Returns
    ///
    /// `Ok(())` if data was sent successfully
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Network is unavailable
    /// - Send buffer is full
    /// - Transport is closed
    async fn send_to(&mut self, data: &[u8], addr: IpEndpoint) -> Result<()>;

    /// Receive data from the network.
    ///
    /// This method blocks until data is available or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to store received data
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - Number of bytes received
    /// - Source endpoint (IP + port)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Receive operation fails
    /// - Buffer is too small
    /// - Transport is closed
    async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, IpEndpoint)>;

    /// Check if the transport is currently connected/ready.
    ///
    /// Default implementation returns `true`. Override if your transport
    /// has connection state to track.
    fn is_ready(&self) -> bool {
        true
    }

    /// Close the transport and release resources.
    ///
    /// Default implementation does nothing. Override if your transport
    /// needs cleanup.
    fn close(&mut self) {
        // Default: no-op
    }
}
