//! Network types for KNX communication.
//!
//! This module provides ergonomic types for working with IP addresses
//! in a `no_std` environment, following the `impl Into<T>` pattern
//! for better API ergonomics.

pub mod transport;

// Embassy adapter (only when embassy features are enabled)
#[cfg(any(feature = "embassy-rp", feature = "embassy-rp-usb"))]
pub mod embassy_adapter;

// Mock transport (only in tests or when std is available)
#[cfg(any(test, feature = "std"))]
pub mod mock_transport;

use core::fmt;

/// IPv4 address representation.
///
/// A lightweight wrapper around a 4-byte array that provides
/// ergonomic conversions from various input types.
///
/// # Examples
///
/// ```
/// use knx_pico::net::Ipv4Addr;
///
/// // From array
/// let addr = Ipv4Addr::from([192, 168, 1, 10]);
///
/// // From tuple
/// let addr = Ipv4Addr::from((192, 168, 1, 10));
///
/// // From raw bytes
/// let addr = Ipv4Addr::new(192, 168, 1, 10);
///
/// // All these work with APIs that accept `impl Into<Ipv4Addr>`
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ipv4Addr {
    octets: [u8; 4],
}

impl Ipv4Addr {
    /// Create a new IPv4 address from individual octets.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_pico::net::Ipv4Addr;
    ///
    /// let addr = Ipv4Addr::new(192, 168, 1, 10);
    /// assert_eq!(addr.octets(), [192, 168, 1, 10]);
    /// ```
    #[inline]
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            octets: [a, b, c, d],
        }
    }

    /// Returns the four octets that make up this address.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_pico::net::Ipv4Addr;
    ///
    /// let addr = Ipv4Addr::new(192, 168, 1, 10);
    /// assert_eq!(addr.octets(), [192, 168, 1, 10]);
    /// ```
    #[inline]
    pub const fn octets(&self) -> [u8; 4] {
        self.octets
    }

    /// Create an unspecified IPv4 address (0.0.0.0).
    ///
    /// This is commonly used for NAT mode in KNXnet/IP.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_pico::net::Ipv4Addr;
    ///
    /// let addr = Ipv4Addr::UNSPECIFIED;
    /// assert_eq!(addr.octets(), [0, 0, 0, 0]);
    /// ```
    pub const UNSPECIFIED: Self = Self::new(0, 0, 0, 0);

    /// Create a localhost IPv4 address (127.0.0.1).
    pub const LOCALHOST: Self = Self::new(127, 0, 0, 1);
}

impl From<[u8; 4]> for Ipv4Addr {
    #[inline]
    fn from(octets: [u8; 4]) -> Self {
        Self { octets }
    }
}

impl From<(u8, u8, u8, u8)> for Ipv4Addr {
    #[inline]
    fn from((a, b, c, d): (u8, u8, u8, u8)) -> Self {
        Self::new(a, b, c, d)
    }
}

impl From<Ipv4Addr> for [u8; 4] {
    #[inline]
    fn from(addr: Ipv4Addr) -> [u8; 4] {
        addr.octets
    }
}

impl From<u32> for Ipv4Addr {
    #[inline]
    fn from(ip: u32) -> Self {
        Self {
            octets: ip.to_be_bytes(),
        }
    }
}

impl From<Ipv4Addr> for u32 {
    #[inline]
    fn from(addr: Ipv4Addr) -> u32 {
        u32::from_be_bytes(addr.octets)
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.octets[0], self.octets[1], self.octets[2], self.octets[3]
        )
    }
}

impl core::str::FromStr for Ipv4Addr {
    type Err = crate::error::KnxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let mut octets = [0u8; 4];

        for octet in &mut octets {
            let part = parts
                .next()
                .ok_or_else(crate::error::KnxError::invalid_address)?;
            *octet = part
                .parse()
                .map_err(|_| crate::error::KnxError::invalid_address())?;
        }

        // Ensure no extra parts
        if parts.next().is_some() {
            return Err(crate::error::KnxError::invalid_address());
        }

        Ok(Self { octets })
    }
}

/// IP endpoint (address + port) for network communication.
///
/// Represents a complete network endpoint with both IP address and port number.
/// Used primarily for UDP socket operations in KNXnet/IP communication.
///
/// # Examples
///
/// ```
/// use knx_pico::net::{IpEndpoint, Ipv4Addr};
///
/// let endpoint = IpEndpoint::new(Ipv4Addr::new(192, 168, 1, 10), 3671);
/// assert_eq!(endpoint.addr.octets(), [192, 168, 1, 10]);
/// assert_eq!(endpoint.port, 3671);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IpEndpoint {
    /// IP address
    pub addr: Ipv4Addr,
    /// Port number
    pub port: u16,
}

impl IpEndpoint {
    /// Create a new IP endpoint.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_pico::net::{IpEndpoint, Ipv4Addr};
    ///
    /// let endpoint = IpEndpoint::new(Ipv4Addr::new(192, 168, 1, 10), 3671);
    /// ```
    #[inline]
    pub const fn new(addr: Ipv4Addr, port: u16) -> Self {
        Self { addr, port }
    }

    /// Create an unspecified endpoint (0.0.0.0:0).
    ///
    /// Useful for NAT mode or when the endpoint is not yet known.
    pub const UNSPECIFIED: Self = Self {
        addr: Ipv4Addr::UNSPECIFIED,
        port: 0,
    };
}

impl Default for IpEndpoint {
    fn default() -> Self {
        Self::UNSPECIFIED
    }
}

impl From<([u8; 4], u16)> for IpEndpoint {
    #[inline]
    fn from((addr, port): ([u8; 4], u16)) -> Self {
        Self {
            addr: Ipv4Addr::from(addr),
            port,
        }
    }
}

impl From<(Ipv4Addr, u16)> for IpEndpoint {
    #[inline]
    fn from((addr, port): (Ipv4Addr, u16)) -> Self {
        Self { addr, port }
    }
}

impl fmt::Display for IpEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let addr = Ipv4Addr::new(192, 168, 1, 10);
        assert_eq!(addr.octets(), [192, 168, 1, 10]);
    }

    #[test]
    fn test_from_array() {
        let addr = Ipv4Addr::from([192, 168, 1, 10]);
        assert_eq!(addr.octets(), [192, 168, 1, 10]);
    }

    #[test]
    fn test_from_tuple() {
        let addr = Ipv4Addr::from((192, 168, 1, 10));
        assert_eq!(addr.octets(), [192, 168, 1, 10]);
    }

    #[test]
    fn test_from_u32() {
        let addr = Ipv4Addr::from(0xC0A8010A); // 192.168.1.10
        assert_eq!(addr.octets(), [192, 168, 1, 10]);
    }

    #[test]
    fn test_to_u32() {
        let addr = Ipv4Addr::new(192, 168, 1, 10);
        assert_eq!(u32::from(addr), 0xC0A8010A);
    }

    #[test]
    fn test_display() {
        let addr = Ipv4Addr::new(192, 168, 1, 10);
        assert_eq!(format!("{}", addr), "192.168.1.10");
    }

    #[test]
    fn test_from_str() {
        let addr: Ipv4Addr = "192.168.1.10".parse().unwrap();
        assert_eq!(addr.octets(), [192, 168, 1, 10]);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("192.168.1".parse::<Ipv4Addr>().is_err());
        assert!("192.168.1.256".parse::<Ipv4Addr>().is_err());
        assert!("192.168.1.10.5".parse::<Ipv4Addr>().is_err());
        assert!("a.b.c.d".parse::<Ipv4Addr>().is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(Ipv4Addr::UNSPECIFIED.octets(), [0, 0, 0, 0]);
        assert_eq!(Ipv4Addr::LOCALHOST.octets(), [127, 0, 0, 1]);
    }
}
