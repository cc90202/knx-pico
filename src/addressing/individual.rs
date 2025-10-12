//! KNX Individual Address implementation.
//!
//! Individual addresses identify physical devices on the KNX bus.
//! Format: Area.Line.Device (e.g., 1.1.5)
//! - Area: 0-15 (4 bits)
//! - Line: 0-15 (4 bits)
//! - Device: 0-255 (8 bits)

use crate::error::{KnxError, Result};
use core::fmt;

/// KNX Individual Address (Area.Line.Device)
///
/// Used to identify physical devices on the KNX bus.
///
/// # Examples
///
/// ```
/// use knx_rs::IndividualAddress;
///
/// // Create from components
/// let addr = IndividualAddress::new(1, 1, 5).unwrap();
/// assert_eq!(addr.to_string(), "1.1.5");
///
/// // Create from raw u16
/// let addr = IndividualAddress::from(0x1105u16);
/// assert_eq!(addr.area(), 1);
/// assert_eq!(addr.line(), 1);
/// assert_eq!(addr.device(), 5);
///
/// // Parse from string
/// let addr: IndividualAddress = "1.1.5".parse().unwrap();
/// assert_eq!(u16::from(addr), 0x1105);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IndividualAddress {
    raw: u16,
}

impl IndividualAddress {
    /// Maximum area value (4 bits)
    pub const MAX_AREA: u8 = 15;
    /// Maximum line value (4 bits)
    pub const MAX_LINE: u8 = 15;
    /// Maximum device value (8 bits)
    pub const MAX_DEVICE: u8 = 255;

    /// Create a new Individual Address from components.
    ///
    /// # Arguments
    ///
    /// * `area` - Area (0-15)
    /// * `line` - Line (0-15)
    /// * `device` - Device (0-255)
    ///
    /// # Errors
    ///
    /// Returns `KnxError::AddressOutOfRange` if any component is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_rs::IndividualAddress;
    ///
    /// let addr = IndividualAddress::new(1, 1, 5)?;
    /// assert_eq!(addr.to_string(), "1.1.5");
    /// # Ok::<(), knx_rs::KnxError>(())
    /// ```
    pub fn new(area: u8, line: u8, device: u8) -> Result<Self> {
        if area > Self::MAX_AREA {
            return Err(KnxError::address_out_of_range());
        }
        if line > Self::MAX_LINE {
            return Err(KnxError::address_out_of_range());
        }
        // device is u8, so it's always in range

        let raw = (u16::from(area) << 12) | (u16::from(line) << 8) | u16::from(device);
        Ok(Self { raw })
    }

    /// Create from a 3-element array `[area, line, device]`.
    ///
    /// Convenient for creating addresses from array literals.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_rs::IndividualAddress;
    ///
    /// let addr = IndividualAddress::from_array([1, 1, 5])?;
    /// assert_eq!(addr.to_string(), "1.1.5");
    /// # Ok::<(), knx_rs::KnxError>(())
    /// ```
    pub fn from_array(parts: [u8; 3]) -> Result<Self> {
        Self::new(parts[0], parts[1], parts[2])
    }

    /// Get the raw u16 representation of the address.
    #[inline(always)]
    pub const fn raw(self) -> u16 {
        self.raw
    }

    /// Get the area component (0-15).
    #[inline(always)]
    pub const fn area(self) -> u8 {
        ((self.raw >> 12) & 0x0F) as u8
    }

    /// Get the line component (0-15).
    #[inline(always)]
    pub const fn line(self) -> u8 {
        ((self.raw >> 8) & 0x0F) as u8
    }

    /// Get the device component (0-255).
    #[inline(always)]
    pub const fn device(self) -> u8 {
        (self.raw & 0xFF) as u8
    }

    /// Encode the address into a byte buffer (big-endian).
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to write to (must be at least 2 bytes)
    ///
    /// # Errors
    ///
    /// Returns `KnxError::BufferTooSmall` if buffer is too small.
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 2 {
            return Err(KnxError::buffer_too_small());
        }
        buf[0..2].copy_from_slice(&self.raw.to_be_bytes());
        Ok(2)
    }

    /// Decode an address from a byte buffer (big-endian).
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to read from (must be at least 2 bytes)
    ///
    /// # Errors
    ///
    /// Returns `KnxError::BufferTooSmall` if buffer is too small.
    #[inline]
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < 2 {
            return Err(KnxError::buffer_too_small());
        }
        let raw = u16::from_be_bytes([buf[0], buf[1]]);
        Ok(Self { raw })
    }
}

impl fmt::Display for IndividualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.area(), self.line(), self.device())
    }
}

impl From<u16> for IndividualAddress {
    #[inline(always)]
    fn from(raw: u16) -> Self {
        Self { raw }
    }
}

impl From<IndividualAddress> for u16 {
    #[inline(always)]
    fn from(addr: IndividualAddress) -> u16 {
        addr.raw
    }
}

impl core::str::FromStr for IndividualAddress {
    type Err = KnxError;

    fn from_str(s: &str) -> Result<Self> {
        // Zero-allocation parsing using iterators
        let mut parts = s.split('.');

        let area = parts
            .next()
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or_else(KnxError::invalid_individual_address)?;

        let line = parts
            .next()
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or_else(KnxError::invalid_individual_address)?;

        let device = parts
            .next()
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or_else(KnxError::invalid_individual_address)?;

        // Ensure no extra parts
        if parts.next().is_some() {
            return Err(KnxError::invalid_individual_address());
        }

        Self::new(area, line, device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let addr = IndividualAddress::new(1, 2, 3).unwrap();
        assert_eq!(addr.area(), 1);
        assert_eq!(addr.line(), 2);
        assert_eq!(addr.device(), 3);
    }

    #[test]
    fn test_new_invalid_area() {
        let result = IndividualAddress::new(16, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_invalid_line() {
        let result = IndividualAddress::new(0, 16, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_raw() {
        let addr = IndividualAddress::from(0x1203u16);
        assert_eq!(addr.area(), 1);
        assert_eq!(addr.line(), 2);
        assert_eq!(addr.device(), 3);
    }

    #[test]
    fn test_to_raw() {
        let addr = IndividualAddress::new(1, 2, 3).unwrap();
        assert_eq!(u16::from(addr), 0x1203);
    }

    #[test]
    fn test_encode_decode() {
        let addr = IndividualAddress::new(15, 15, 255).unwrap();
        let mut buf = [0u8; 2];
        addr.encode(&mut buf).unwrap();
        let decoded = IndividualAddress::decode(&buf).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn test_display() {
        let addr = IndividualAddress::new(1, 2, 3).unwrap();
        assert_eq!(format!("{}", addr), "1.2.3");
    }

    #[test]
    fn test_from_str() {
        let addr: IndividualAddress = "1.2.3".parse().unwrap();
        assert_eq!(addr.area(), 1);
        assert_eq!(addr.line(), 2);
        assert_eq!(addr.device(), 3);
    }

    #[test]
    fn test_from_str_invalid() {
        // Too few parts
        let result = "1.2".parse::<IndividualAddress>();
        assert!(result.is_err());

        // Out of range
        let result = "16.0.0".parse::<IndividualAddress>();
        assert!(result.is_err());

        // Too many parts
        let result = "1.2.3.4".parse::<IndividualAddress>();
        assert!(result.is_err());

        // Non-numeric
        let result = "a.b.c".parse::<IndividualAddress>();
        assert!(result.is_err());

        // Empty
        let result = "".parse::<IndividualAddress>();
        assert!(result.is_err());
    }
}
