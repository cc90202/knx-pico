//! Embassy UDP socket adapter for `AsyncTransport`.
//!
//! This module provides an adapter that wraps `embassy_net::UdpSocket`
//! to implement the `AsyncTransport` trait, enabling its use with
//! `AsyncTunnelClient` and other transport-agnostic components.
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::net::embassy_adapter::EmbassyUdpTransport;
//! use knx_pico::net::IpEndpoint;
//! use embassy_net::Stack;
//!
//! let mut transport = EmbassyUdpTransport::new(
//!     &stack,
//!     &mut rx_meta,
//!     &mut rx_buffer,
//!     &mut tx_meta,
//!     &mut tx_buffer,
//! );
//!
//! transport.bind(0).await?;  // Bind to any port
//! transport.send_to(&data, remote_endpoint).await?;
//! ```

use crate::error::{KnxError, Result};
use crate::net::transport::AsyncTransport;
use crate::net::{Ipv4Addr, IpEndpoint};
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpAddress, IpEndpoint as EmbassyEndpoint, Stack,
};

/// Adapter for `embassy_net::UdpSocket` implementing `AsyncTransport`.
///
/// This adapter wraps an Embassy UDP socket and provides the `AsyncTransport`
/// interface, allowing it to be used with any component that depends on the
/// transport abstraction.
///
/// # Lifetimes
///
/// - `'a` - Lifetime of the network stack
/// - `'b` - Lifetime of the metadata and buffer arrays
///
/// # Type Parameters
///
/// - `D` - Network driver type (e.g., `cyw43::NetDriver`)
///
/// # Examples
///
/// ```rust,no_run
/// use knx_pico::net::embassy_adapter::EmbassyUdpTransport;
/// use embassy_net::Stack;
/// use embassy_net::udp::PacketMetadata;
///
/// const RX_META_SIZE: usize = 4;
/// const RX_BUFFER_SIZE: usize = 2048;
/// const TX_META_SIZE: usize = 4;
/// const TX_BUFFER_SIZE: usize = 2048;
///
/// let mut rx_meta = [PacketMetadata::EMPTY; RX_META_SIZE];
/// let mut rx_buffer = [0u8; RX_BUFFER_SIZE];
/// let mut tx_meta = [PacketMetadata::EMPTY; TX_META_SIZE];
/// let mut tx_buffer = [0u8; TX_BUFFER_SIZE];
///
/// let transport = EmbassyUdpTransport::new(
///     &stack,
///     &mut rx_meta,
///     &mut rx_buffer,
///     &mut tx_meta,
///     &mut tx_buffer,
/// );
/// ```
pub struct EmbassyUdpTransport<'a, 'b, D: embassy_net::driver::Driver> {
    socket: UdpSocket<'a>,
    _phantom: core::marker::PhantomData<(&'b (), D)>,
}

impl<'a, 'b, D: embassy_net::driver::Driver> EmbassyUdpTransport<'a, 'b, D> {
    /// Create a new Embassy UDP transport adapter.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack
    /// * `rx_meta` - Receive metadata buffer (typically 4-8 entries)
    /// * `rx_buffer` - Receive data buffer (typically 2048 bytes)
    /// * `tx_meta` - Transmit metadata buffer (typically 4-8 entries)
    /// * `tx_buffer` - Transmit data buffer (typically 2048 bytes)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let transport = EmbassyUdpTransport::new(
    ///     &stack,
    ///     &mut rx_meta,
    ///     &mut rx_buffer,
    ///     &mut tx_meta,
    ///     &mut tx_buffer,
    /// );
    /// ```
    pub fn new(
        stack: &'a Stack<D>,
        rx_meta: &'b mut [PacketMetadata],
        rx_buffer: &'b mut [u8],
        tx_meta: &'b mut [PacketMetadata],
        tx_buffer: &'b mut [u8],
    ) -> Self {
        let socket = UdpSocket::new(stack, rx_meta, rx_buffer, tx_meta, tx_buffer);
        Self {
            socket,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Bind the socket to a local port.
    ///
    /// # Arguments
    ///
    /// * `port` - Local port to bind to (0 = any available port)
    ///
    /// # Errors
    ///
    /// Returns error if the port is already in use or binding fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// transport.bind(0).await?;  // Bind to any available port
    /// transport.bind(3671).await?;  // Bind to specific KNX port
    /// ```
    pub async fn bind(&mut self, port: u16) -> Result<()> {
        self.socket
            .bind(port)
            .map_err(|_| KnxError::socket_error())
    }
}

impl<'a, 'b, D: embassy_net::driver::Driver> AsyncTransport for EmbassyUdpTransport<'a, 'b, D> {
    async fn send_to(&mut self, data: &[u8], addr: IpEndpoint) -> Result<()> {
        // Convert our IpEndpoint to embassy IpEndpoint
        let embassy_addr = convert_to_embassy_endpoint(addr);

        self.socket
            .send_to(data, embassy_addr)
            .await
            .map_err(|_| KnxError::send_failed())
    }

    async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, IpEndpoint)> {
        let (n, embassy_addr) = self
            .socket
            .recv_from(buf)
            .await
            .map_err(|_| KnxError::receive_failed())?;

        // Convert embassy IpEndpoint to our IpEndpoint
        let addr = convert_from_embassy_endpoint(embassy_addr);

        Ok((n, addr))
    }

    fn is_ready(&self) -> bool {
        // Socket is ready if it's bound (has endpoint)
        self.socket.endpoint().is_some()
    }

    fn close(&mut self) {
        self.socket.close();
    }
}

/// Convert from our `IpEndpoint` to embassy `IpEndpoint`.
#[inline]
fn convert_to_embassy_endpoint(endpoint: IpEndpoint) -> EmbassyEndpoint {
    let octets = endpoint.addr.octets();
    EmbassyEndpoint::new(
        IpAddress::v4(octets[0], octets[1], octets[2], octets[3]),
        endpoint.port,
    )
}

/// Convert from embassy `IpEndpoint` to our `IpEndpoint`.
#[inline]
fn convert_from_embassy_endpoint(endpoint: EmbassyEndpoint) -> IpEndpoint {
    match endpoint.addr {
        IpAddress::Ipv4(addr) => {
            let octets = addr.as_bytes();
            IpEndpoint::new(Ipv4Addr::from([octets[0], octets[1], octets[2], octets[3]]), endpoint.port)
        }
        // For IPv6, we'll just return unspecified for now
        // KNXnet/IP doesn't use IPv6
        _ => IpEndpoint::UNSPECIFIED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_conversion() {
        let our_endpoint = IpEndpoint::new(Ipv4Addr::new(192, 168, 1, 10), 3671);
        let embassy_endpoint = convert_to_embassy_endpoint(our_endpoint);
        let converted_back = convert_from_embassy_endpoint(embassy_endpoint);

        assert_eq!(our_endpoint, converted_back);
    }

    #[test]
    fn test_endpoint_conversion_unspecified() {
        let our_endpoint = IpEndpoint::UNSPECIFIED;
        let embassy_endpoint = convert_to_embassy_endpoint(our_endpoint);
        let converted_back = convert_from_embassy_endpoint(embassy_endpoint);

        assert_eq!(our_endpoint, converted_back);
    }
}
