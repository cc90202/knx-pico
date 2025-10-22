//! KNX Group Address implementation.
//!
//! Group addresses represent logical groupings of devices for functional control.
//! Two formats are supported:
//! - 2-level: Main/Sub (e.g., 1/234)
//! - 3-level: Main/Middle/Sub (e.g., 1/2/3) - most common
//!
//! Internally stored as 16 bits:
//! - Main: 5 bits (0-31)
//! - Middle: 3 bits (0-7)
//! - Sub: 8 bits (0-255)

use crate::error::{KnxError, Result};
use core::fmt;

/// KNX Group Address
///
/// Used for logical grouping of devices and functions.
///
/// # Examples
///
/// ```
/// use knx_pico::GroupAddress;
///
/// // Create 3-level address
/// let addr = GroupAddress::new(1, 2, 3).unwrap();
/// assert_eq!(addr.to_string(), "1/2/3");
///
/// // Create 2-level address
/// let addr = GroupAddress::new_2level(1, 234).unwrap();
/// assert_eq!(addr.to_string_2level(), "1/234");
///
/// // Create from raw u16
/// let addr = GroupAddress::from(0x0A03u16);
/// assert_eq!(addr.main(), 1);
/// assert_eq!(addr.middle(), 2);
/// assert_eq!(addr.sub(), 3);
///
/// // Parse from string (auto-detects format)
/// let addr: GroupAddress = "1/2/3".parse().unwrap();
/// assert_eq!(u16::from(addr), 0x0A03);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GroupAddress {
    raw: u16,
}

impl GroupAddress {
    /// Maximum main group value (5 bits)
    pub const MAX_MAIN: u8 = 31;
    /// Maximum middle group value (3 bits)
    pub const MAX_MIDDLE: u8 = 7;
    /// Maximum sub group value (8 bits)
    pub const MAX_SUB: u8 = 255;
    /// Maximum sub value for 2-level format (11 bits)
    pub const MAX_SUB_2LEVEL: u16 = 2047;

    /// Create a new 3-level Group Address (Main/Middle/Sub).
    ///
    /// # Arguments
    ///
    /// * `main` - Main group (0-31)
    /// * `middle` - Middle group (0-7)
    /// * `sub` - Sub group (0-255)
    ///
    /// # Errors
    ///
    /// Returns `KnxError::AddressOutOfRange` if any component is out of range.
    pub fn new(main: u8, middle: u8, sub: u8) -> Result<Self> {
        if main > Self::MAX_MAIN {
            return Err(KnxError::address_out_of_range());
        }
        if middle > Self::MAX_MIDDLE {
            return Err(KnxError::address_out_of_range());
        }
        // sub is u8, so it's always in range

        let raw = (u16::from(main) << 11) | (u16::from(middle) << 8) | u16::from(sub);
        Ok(Self { raw })
    }

    /// Create a new 2-level Group Address (Main/Sub).
    ///
    /// # Arguments
    ///
    /// * `main` - Main group (0-31)
    /// * `sub` - Sub group (0-2047)
    ///
    /// # Errors
    ///
    /// Returns `KnxError::AddressOutOfRange` if any component is out of range.
    pub fn new_2level(main: u8, sub: u16) -> Result<Self> {
        if main > Self::MAX_MAIN {
            return Err(KnxError::address_out_of_range());
        }
        if sub > Self::MAX_SUB_2LEVEL {
            return Err(KnxError::address_out_of_range());
        }

        let raw = (u16::from(main) << 11) | sub;
        Ok(Self { raw })
    }

    /// Create from a 3-element array `[main, middle, sub]`.
    ///
    /// Convenient for creating 3-level addresses from array literals.
    ///
    /// # Examples
    ///
    /// ```
    /// use knx_pico::GroupAddress;
    ///
    /// let addr = GroupAddress::from_array([1, 2, 3])?;
    /// assert_eq!(addr.to_string(), "1/2/3");
    /// # Ok::<(), knx_pico::KnxError>(())
    /// ```
    pub fn from_array(parts: [u8; 3]) -> Result<Self> {
        Self::new(parts[0], parts[1], parts[2])
    }

    /// Get the raw u16 representation of the address.
    #[inline(always)]
    pub const fn raw(self) -> u16 {
        self.raw
    }

    /// Get the main group component (0-31).
    #[inline(always)]
    pub const fn main(self) -> u8 {
        ((self.raw >> 11) & 0x1F) as u8
    }

    /// Get the middle group component for 3-level format (0-7).
    #[inline(always)]
    pub const fn middle(self) -> u8 {
        ((self.raw >> 8) & 0x07) as u8
    }

    /// Get the sub group component for 3-level format (0-255).
    #[inline(always)]
    pub const fn sub(self) -> u8 {
        (self.raw & 0xFF) as u8
    }

    /// Get the sub group component for 2-level format (0-2047).
    #[inline(always)]
    pub const fn sub_2level(self) -> u16 {
        self.raw & 0x07FF
    }

    /// Format as 3-level string (Main/Middle/Sub).
    pub fn to_string_3level(&self) -> heapless::String<16> {
        use core::fmt::Write;
        let mut s = heapless::String::new();
        let _ = write!(s, "{}/{}/{}", self.main(), self.middle(), self.sub());
        s
    }

    /// Format as 2-level string (Main/Sub).
    pub fn to_string_2level(&self) -> heapless::String<16> {
        use core::fmt::Write;
        let mut s = heapless::String::new();
        let _ = write!(s, "{}/{}", self.main(), self.sub_2level());
        s
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

impl From<u16> for GroupAddress {
    #[inline(always)]
    fn from(raw: u16) -> Self {
        Self { raw }
    }
}

impl From<GroupAddress> for u16 {
    #[inline(always)]
    fn from(addr: GroupAddress) -> u16 {
        addr.raw
    }
}

impl fmt::Display for GroupAddress {
    /// Format as 3-level address by default
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.main(), self.middle(), self.sub())
    }
}

impl core::str::FromStr for GroupAddress {
    type Err = KnxError;

    fn from_str(s: &str) -> Result<Self> {
        // Zero-allocation parsing using iterators
        let mut parts = s.split('/');

        let main = parts
            .next()
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or_else(KnxError::invalid_group_address)?;

        let middle = parts
            .next()
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or_else(KnxError::invalid_group_address)?;

        // Check if there's a third part (3-level format)
        if let Some(sub_str) = parts.next() {
            // 3-level format: Main/Middle/Sub
            let sub = sub_str
                .parse::<u8>()
                .ok()
                .ok_or_else(KnxError::invalid_group_address)?;

            // Ensure no extra parts
            if parts.next().is_some() {
                return Err(KnxError::invalid_group_address());
            }

            // middle is actually middle (u8), not sub (u16)
            if middle > 255 {
                return Err(KnxError::invalid_group_address());
            }

            Self::new(main, middle as u8, sub)
        } else {
            // 2-level format: Main/Sub
            // middle is actually the sub value
            // Ensure no extra parts
            if parts.next().is_some() {
                return Err(KnxError::invalid_group_address());
            }

            Self::new_2level(main, middle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_3level_valid() {
        let addr = GroupAddress::new(1, 2, 3).unwrap();
        assert_eq!(addr.main(), 1);
        assert_eq!(addr.middle(), 2);
        assert_eq!(addr.sub(), 3);
    }

    #[test]
    fn test_new_3level_invalid_main() {
        let result = GroupAddress::new(32, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_3level_invalid_middle() {
        let result = GroupAddress::new(0, 8, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_2level_valid() {
        let addr = GroupAddress::new_2level(1, 234).unwrap();
        assert_eq!(addr.main(), 1);
        assert_eq!(addr.sub_2level(), 234);
    }

    #[test]
    fn test_new_2level_invalid() {
        let result = GroupAddress::new_2level(0, 2048);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_raw() {
        // 1/2/3 = 0b00001_010_00000011 = 0x0A03
        let addr = GroupAddress::from(0x0A03u16);
        assert_eq!(addr.main(), 1);
        assert_eq!(addr.middle(), 2);
        assert_eq!(addr.sub(), 3);
    }

    #[test]
    fn test_to_raw() {
        let addr = GroupAddress::new(1, 2, 3).unwrap();
        assert_eq!(u16::from(addr), 0x0A03);
    }

    #[test]
    fn test_encode_decode() {
        let addr = GroupAddress::new(31, 7, 255).unwrap();
        let mut buf = [0u8; 2];
        addr.encode(&mut buf).unwrap();
        let decoded = GroupAddress::decode(&buf).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn test_display_3level() {
        let addr = GroupAddress::new(1, 2, 3).unwrap();
        assert_eq!(format!("{}", addr), "1/2/3");
    }

    #[test]
    fn test_to_string_2level() {
        let addr = GroupAddress::new_2level(1, 234).unwrap();
        assert_eq!(addr.to_string_2level(), "1/234");
    }

    #[test]
    fn test_from_str_3level() {
        let addr: GroupAddress = "1/2/3".parse().unwrap();
        assert_eq!(addr.main(), 1);
        assert_eq!(addr.middle(), 2);
        assert_eq!(addr.sub(), 3);
    }

    #[test]
    fn test_from_str_2level() {
        let addr: GroupAddress = "1/234".parse().unwrap();
        assert_eq!(addr.main(), 1);
        assert_eq!(addr.sub_2level(), 234);
    }

    #[test]
    fn test_from_str_invalid() {
        // Too few parts
        let result = "1".parse::<GroupAddress>();
        assert!(result.is_err());

        // Out of range (main)
        let result = "32/0/0".parse::<GroupAddress>();
        assert!(result.is_err());

        // Too many parts
        let result = "1/2/3/4".parse::<GroupAddress>();
        assert!(result.is_err());

        // Non-numeric
        let result = "a/b/c".parse::<GroupAddress>();
        assert!(result.is_err());

        // Empty
        let result = "".parse::<GroupAddress>();
        assert!(result.is_err());

        // Out of range (2-level sub)
        let result = "1/2048".parse::<GroupAddress>();
        assert!(result.is_err());
    }
}
