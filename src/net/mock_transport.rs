//! Mock transport implementation for testing.
//!
//! This module provides a mock implementation of `AsyncTransport` that can be used
//! in unit tests to simulate network behavior without requiring actual network hardware.
//!
//! ## Example
//!
//! ```rust
//! use knx_pico::net::mock_transport::MockTransport;
//! use knx_pico::net::transport::AsyncTransport;
//! use knx_pico::net::IpEndpoint;
//!
//! #[tokio::test]
//! async fn test_knx_communication() {
//!     let mut mock = MockTransport::new();
//!
//!     // Program expected responses
//!     mock.add_response(vec![0x06, 0x10, 0x02, 0x06, ...]);  // CONNECT_RESPONSE
//!
//!     // Use with AsyncTunnelClient
//!     let client = AsyncTunnelClient::new(mock, gateway_addr);
//!     client.connect().await?;
//!
//!     // Verify what was sent
//!     assert_eq!(mock.sent_packets().len(), 1);
//!     assert_eq!(mock.sent_packets()[0], expected_connect_request);
//! }
//! ```

use crate::error::Result;
use crate::net::transport::AsyncTransport;
use crate::net::IpEndpoint;

#[cfg(feature = "std")]
use std::collections::VecDeque;

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Mock transport for testing KNX communication without real network.
///
/// This mock transport allows you to:
/// - Pre-program responses that will be returned by `recv_from()`
/// - Inspect packets sent via `send_to()`
/// - Simulate network errors
/// - Test protocol logic in isolation
///
/// # Examples
///
/// ```rust
/// use knx_pico::net::mock_transport::MockTransport;
/// use knx_pico::net::IpEndpoint;
///
/// let mut mock = MockTransport::new();
///
/// // Program a response
/// mock.add_response(vec![0x06, 0x10, 0x02, 0x06, 0x00, 0x0E]);
///
/// // Simulate receiving
/// let mut buf = [0u8; 512];
/// let (n, addr) = mock.recv_from(&mut buf).await?;
/// assert_eq!(&buf[..n], &[0x06, 0x10, 0x02, 0x06, 0x00, 0x0E]);
/// ```
#[derive(Debug, Default)]
pub struct MockTransport {
    /// Queue of pre-programmed responses to return from recv_from()
    responses: VecDeque<(Vec<u8>, IpEndpoint)>,
    /// Record of all packets sent via send_to()
    sent_packets: Vec<(Vec<u8>, IpEndpoint)>,
    /// Whether the transport is "ready" (simulates binding)
    ready: bool,
}

impl MockTransport {
    /// Create a new mock transport.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mock = MockTransport::new();
    /// ```
    pub fn new() -> Self {
        Self {
            responses: VecDeque::new(),
            sent_packets: Vec::new(),
            ready: true, // Start ready by default
        }
    }

    /// Add a response that will be returned by the next `recv_from()` call.
    ///
    /// Responses are returned in FIFO order.
    ///
    /// # Arguments
    ///
    /// * `data` - Response data to return
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut mock = MockTransport::new();
    /// mock.add_response(vec![0x06, 0x10, 0x02, 0x06]);
    /// ```
    pub fn add_response(&mut self, data: Vec<u8>) {
        self.add_response_from(data, IpEndpoint::new([192, 168, 1, 10].into(), 3671));
    }

    /// Add a response with specific source endpoint.
    ///
    /// # Arguments
    ///
    /// * `data` - Response data to return
    /// * `from` - Source endpoint to report
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut mock = MockTransport::new();
    /// let gateway = IpEndpoint::new([192, 168, 1, 10].into(), 3671);
    /// mock.add_response_from(vec![0x06, 0x10, 0x02, 0x06], gateway);
    /// ```
    pub fn add_response_from(&mut self, data: Vec<u8>, from: IpEndpoint) {
        self.responses.push_back((data, from));
    }

    /// Get all packets that were sent via `send_to()`.
    ///
    /// Returns a slice of `(data, destination)` tuples.
    ///
    /// # Examples
    ///
    /// ```rust
    /// assert_eq!(mock.sent_packets().len(), 2);
    /// assert_eq!(mock.sent_packets()[0].0, expected_connect_request);
    /// ```
    pub fn sent_packets(&self) -> &[(Vec<u8>, IpEndpoint)] {
        &self.sent_packets
    }

    /// Get the last packet that was sent.
    ///
    /// Returns `None` if no packets have been sent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// if let Some((data, dest)) = mock.last_sent() {
    ///     assert_eq!(data, expected_packet);
    /// }
    /// ```
    pub fn last_sent(&self) -> Option<&(Vec<u8>, IpEndpoint)> {
        self.sent_packets.last()
    }

    /// Clear all sent packets from the history.
    ///
    /// Useful for resetting state between test phases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// mock.clear_sent();
    /// assert_eq!(mock.sent_packets().len(), 0);
    /// ```
    pub fn clear_sent(&mut self) {
        self.sent_packets.clear();
    }

    /// Clear all pending responses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// mock.clear_responses();
    /// ```
    pub fn clear_responses(&mut self) {
        self.responses.clear();
    }

    /// Set whether the transport should report as "ready".
    ///
    /// # Examples
    ///
    /// ```rust
    /// mock.set_ready(false);  // Simulate unbound socket
    /// assert!(!mock.is_ready());
    /// ```
    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    /// Check if there are pending responses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// mock.add_response(vec![0x06, 0x10]);
    /// assert!(mock.has_responses());
    /// ```
    pub fn has_responses(&self) -> bool {
        !self.responses.is_empty()
    }

    /// Get the number of pending responses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// assert_eq!(mock.pending_responses(), 3);
    /// ```
    pub fn pending_responses(&self) -> usize {
        self.responses.len()
    }
}

impl AsyncTransport for MockTransport {
    async fn send_to(&mut self, data: &[u8], addr: IpEndpoint) -> Result<()> {
        // Record the sent packet
        self.sent_packets.push((data.to_vec(), addr));
        Ok(())
    }

    async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, IpEndpoint)> {
        // Return the next pre-programmed response
        if let Some((data, from)) = self.responses.pop_front() {
            let len = data.len().min(buf.len());
            buf[..len].copy_from_slice(&data[..len]);
            Ok((len, from))
        } else {
            // No more responses - simulate timeout/error
            Err(crate::error::KnxError::Timeout)
        }
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    fn close(&mut self) {
        self.ready = false;
        self.responses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_mock_send_receive() {
        let mut mock = MockTransport::new();

        // Add response
        mock.add_response(vec![0x01, 0x02, 0x03]);

        // Send data
        let dest = IpEndpoint::new([192, 168, 1, 10].into(), 3671);
        mock.send_to(&[0xAA, 0xBB], dest).await.unwrap();

        // Check sent
        assert_eq!(mock.sent_packets().len(), 1);
        assert_eq!(mock.sent_packets()[0].0, vec![0xAA, 0xBB]);
        assert_eq!(mock.sent_packets()[0].1, dest);

        // Receive response
        let mut buf = [0u8; 10];
        let (n, _) = mock.recv_from(&mut buf).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[0x01, 0x02, 0x03]);
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_mock_no_response_returns_error() {
        let mut mock = MockTransport::new();

        // No responses programmed
        let mut buf = [0u8; 10];
        let result = mock.recv_from(&mut buf).await;

        assert!(result.is_err());
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_mock_fifo_order() {
        let mut mock = MockTransport::new();

        mock.add_response(vec![0x01]);
        mock.add_response(vec![0x02]);
        mock.add_response(vec![0x03]);

        let mut buf = [0u8; 10];

        let (_n, _) = mock.recv_from(&mut buf).await.unwrap();
        assert_eq!(buf[0], 0x01);

        let (_n, _) = mock.recv_from(&mut buf).await.unwrap();
        assert_eq!(buf[0], 0x02);

        let (_n, _) = mock.recv_from(&mut buf).await.unwrap();
        assert_eq!(buf[0], 0x03);
    }

    #[test]
    fn test_mock_ready_state() {
        let mut mock = MockTransport::new();
        assert!(mock.is_ready());

        mock.set_ready(false);
        assert!(!mock.is_ready());

        mock.close();
        assert!(!mock.is_ready());
    }
}
