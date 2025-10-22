//! DPT 7.xxx - 2-byte Unsigned Value (2 bytes)
//!
//! 16-bit unsigned datapoint types represent values from 0 to 65535.
//!
//! ## Format
//!
//! - 16 bits: unsigned value (0-65535), big-endian
//!
//! ## Common Subtypes
//!
//! - **7.001** - Pulses (0-65535)
//! - **7.002** - Time Period (ms)
//! - **7.003** - Time Period (10ms)
//! - **7.004** - Time Period (100ms)
//! - **7.005** - Time Period (s)
//! - **7.006** - Time Period (min)
//! - **7.007** - Time Period (h)
//! - **7.010** - Property Data Type
//! - **7.011** - Length (mm)
//! - **7.012** - Current (mA)
//! - **7.013** - Brightness (lux)
//! - **7.600** - Color Temperature (K)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::dpt::{Dpt7, DptDecode};
//!
//! // Decode brightness in lux
//! let lux = Dpt7::Brightness.decode(&[0x13, 0x88])?;  // 5000 lux
//!
//! // Encode pulses
//! let data = Dpt7::Pulses.encode_to_bytes(1234)?;  // [0x04, 0xD2]
//!
//! // Color temperature
//! let kelvin = Dpt7::ColorTemperature.decode(&[0x0F, 0xA0])?;  // 4000K
//! ```

use crate::error::{KnxError, Result};
use crate::dpt::DptDecode;

/// DPT 7.xxx 16-bit unsigned types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt7 {
    /// DPT 7.001 - Pulses (0-65535)
    Pulses,
    /// DPT 7.002 - Time Period (ms)
    TimePeriodMs,
    /// DPT 7.003 - Time Period (10ms)
    TimePeriod10Ms,
    /// DPT 7.004 - Time Period (100ms)
    TimePeriod100Ms,
    /// DPT 7.005 - Time Period (s)
    TimePeriodSec,
    /// DPT 7.006 - Time Period (min)
    TimePeriodMin,
    /// DPT 7.007 - Time Period (h)
    TimePeriodHr,
    /// DPT 7.010 - Property Data Type
    PropDataType,
    /// DPT 7.011 - Length (mm)
    LengthMm,
    /// DPT 7.012 - Current (mA)
    CurrentMa,
    /// DPT 7.013 - Brightness (lux)
    Brightness,
    /// DPT 7.600 - Color Temperature (K)
    ColorTemperature,
}

impl Dpt7 {
    /// Get the DPT identifier string (e.g., "7.001")
    #[inline]
    pub const fn identifier(&self) -> &'static str {
        match self {
            Dpt7::Pulses => "7.001",
            Dpt7::TimePeriodMs => "7.002",
            Dpt7::TimePeriod10Ms => "7.003",
            Dpt7::TimePeriod100Ms => "7.004",
            Dpt7::TimePeriodSec => "7.005",
            Dpt7::TimePeriodMin => "7.006",
            Dpt7::TimePeriodHr => "7.007",
            Dpt7::PropDataType => "7.010",
            Dpt7::LengthMm => "7.011",
            Dpt7::CurrentMa => "7.012",
            Dpt7::Brightness => "7.013",
            Dpt7::ColorTemperature => "7.600",
        }
    }

    /// Get the unit string for this DPT
    #[inline]
    pub const fn unit(&self) -> &'static str {
        match self {
            Dpt7::Pulses => "pulses",
            Dpt7::TimePeriodMs => "ms",
            Dpt7::TimePeriod10Ms => "ms",
            Dpt7::TimePeriod100Ms => "ms",
            Dpt7::TimePeriodSec => "s",
            Dpt7::TimePeriodMin => "min",
            Dpt7::TimePeriodHr => "h",
            Dpt7::PropDataType => "",
            Dpt7::LengthMm => "mm",
            Dpt7::CurrentMa => "mA",
            Dpt7::Brightness => "lux",
            Dpt7::ColorTemperature => "K",
        }
    }

    /// Get the valid range for this DPT (min, max)
    #[inline]
    pub const fn range(&self) -> (u16, u16) {
        // All DPT 7.xxx types use full u16 range
        (0, 65535)
    }

    /// Encode a u16 value to 2 bytes (big-endian)
    ///
    /// # Performance
    /// - Uses optimized big-endian conversion
    /// - Inlined for zero-cost abstraction
    /// - No heap allocations
    ///
    /// # Arguments
    /// * `value` - The value to encode (0-65535)
    ///
    /// # Returns
    /// 2-byte array in big-endian format
    ///
    /// # Example
    /// ```rust,no_run
    /// let bytes = Dpt7::Pulses.encode_to_bytes(1234)?;
    /// assert_eq!(bytes, [0x04, 0xD2]);
    /// ```
    #[inline]
    pub fn encode_to_bytes(&self, value: u16) -> Result<[u8; 2]> {
        // All values 0-65535 are valid for DPT 7.xxx
        // Use to_be_bytes() which compiles to optimal code on all platforms
        Ok(value.to_be_bytes())
    }

    /// Decode 2 bytes (big-endian) to u16 value
    ///
    /// # Performance
    /// - Uses optimized big-endian conversion
    /// - Inlined for zero-cost abstraction
    /// - Bounds check happens once at start
    ///
    /// # Safety
    /// Uses `get_unchecked` after bounds validation for optimal performance.
    /// This is safe because we verify the slice length first.
    ///
    /// # Arguments
    /// * `data` - 2-byte slice in big-endian format
    ///
    /// # Returns
    /// The decoded u16 value (0-65535)
    ///
    /// # Errors
    /// Returns `InvalidDptData` if data length is not exactly 2 bytes
    #[inline]
    fn decode_raw(&self, data: &[u8]) -> Result<u16> {
        if data.len() < 2 {
            return Err(KnxError::invalid_dpt_data());
        }

        // SAFETY: We just validated that data.len() >= 2
        // This eliminates redundant bounds checks
        let bytes = unsafe {
            [
                *data.get_unchecked(0),
                *data.get_unchecked(1),
            ]
        };

        // u16::from_be_bytes is optimized by LLVM to a simple load on big-endian
        // and a bswap instruction on little-endian (single cycle on ARM)
        Ok(u16::from_be_bytes(bytes))
    }
}

impl DptDecode<u16> for Dpt7 {
    #[inline]
    fn decode(&self, data: &[u8]) -> Result<u16> {
        self.decode_raw(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulses_encode() {
        // Zero
        assert_eq!(Dpt7::Pulses.encode_to_bytes(0).unwrap(), [0x00, 0x00]);

        // Small value
        assert_eq!(Dpt7::Pulses.encode_to_bytes(1).unwrap(), [0x00, 0x01]);

        // Medium value
        assert_eq!(Dpt7::Pulses.encode_to_bytes(1234).unwrap(), [0x04, 0xD2]);

        // Large value
        assert_eq!(Dpt7::Pulses.encode_to_bytes(5000).unwrap(), [0x13, 0x88]);

        // Maximum value
        assert_eq!(Dpt7::Pulses.encode_to_bytes(65535).unwrap(), [0xFF, 0xFF]);
    }

    #[test]
    fn test_pulses_decode() {
        // Zero
        assert_eq!(Dpt7::Pulses.decode(&[0x00, 0x00]).unwrap(), 0);

        // Small value
        assert_eq!(Dpt7::Pulses.decode(&[0x00, 0x01]).unwrap(), 1);

        // Medium value
        assert_eq!(Dpt7::Pulses.decode(&[0x04, 0xD2]).unwrap(), 1234);

        // Large value
        assert_eq!(Dpt7::Pulses.decode(&[0x13, 0x88]).unwrap(), 5000);

        // Maximum value
        assert_eq!(Dpt7::Pulses.decode(&[0xFF, 0xFF]).unwrap(), 65535);
    }

    #[test]
    fn test_brightness_encode() {
        // 0 lux (dark)
        assert_eq!(Dpt7::Brightness.encode_to_bytes(0).unwrap(), [0x00, 0x00]);

        // 1000 lux (overcast day)
        assert_eq!(Dpt7::Brightness.encode_to_bytes(1000).unwrap(), [0x03, 0xE8]);

        // 5000 lux (office lighting)
        assert_eq!(Dpt7::Brightness.encode_to_bytes(5000).unwrap(), [0x13, 0x88]);

        // 10000 lux (bright day)
        assert_eq!(Dpt7::Brightness.encode_to_bytes(10000).unwrap(), [0x27, 0x10]);

        // 65535 lux (max)
        assert_eq!(Dpt7::Brightness.encode_to_bytes(65535).unwrap(), [0xFF, 0xFF]);
    }

    #[test]
    fn test_brightness_decode() {
        assert_eq!(Dpt7::Brightness.decode(&[0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt7::Brightness.decode(&[0x03, 0xE8]).unwrap(), 1000);
        assert_eq!(Dpt7::Brightness.decode(&[0x13, 0x88]).unwrap(), 5000);
        assert_eq!(Dpt7::Brightness.decode(&[0x27, 0x10]).unwrap(), 10000);
        assert_eq!(Dpt7::Brightness.decode(&[0xFF, 0xFF]).unwrap(), 65535);
    }

    #[test]
    fn test_color_temperature_encode() {
        // 2700K (warm white)
        assert_eq!(Dpt7::ColorTemperature.encode_to_bytes(2700).unwrap(), [0x0A, 0x8C]);

        // 4000K (neutral white)
        assert_eq!(Dpt7::ColorTemperature.encode_to_bytes(4000).unwrap(), [0x0F, 0xA0]);

        // 6500K (cool white / daylight)
        assert_eq!(Dpt7::ColorTemperature.encode_to_bytes(6500).unwrap(), [0x19, 0x64]);
    }

    #[test]
    fn test_color_temperature_decode() {
        assert_eq!(Dpt7::ColorTemperature.decode(&[0x0A, 0x8C]).unwrap(), 2700);
        assert_eq!(Dpt7::ColorTemperature.decode(&[0x0F, 0xA0]).unwrap(), 4000);
        assert_eq!(Dpt7::ColorTemperature.decode(&[0x19, 0x64]).unwrap(), 6500);
    }

    #[test]
    fn test_time_period_sec_encode() {
        // 0 seconds
        assert_eq!(Dpt7::TimePeriodSec.encode_to_bytes(0).unwrap(), [0x00, 0x00]);

        // 1 minute = 60 seconds
        assert_eq!(Dpt7::TimePeriodSec.encode_to_bytes(60).unwrap(), [0x00, 0x3C]);

        // 1 hour = 3600 seconds
        assert_eq!(Dpt7::TimePeriodSec.encode_to_bytes(3600).unwrap(), [0x0E, 0x10]);

        // ~18 hours (max)
        assert_eq!(Dpt7::TimePeriodSec.encode_to_bytes(65535).unwrap(), [0xFF, 0xFF]);
    }

    #[test]
    fn test_time_period_sec_decode() {
        assert_eq!(Dpt7::TimePeriodSec.decode(&[0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt7::TimePeriodSec.decode(&[0x00, 0x3C]).unwrap(), 60);
        assert_eq!(Dpt7::TimePeriodSec.decode(&[0x0E, 0x10]).unwrap(), 3600);
        assert_eq!(Dpt7::TimePeriodSec.decode(&[0xFF, 0xFF]).unwrap(), 65535);
    }

    #[test]
    fn test_current_ma_encode() {
        // 0 mA
        assert_eq!(Dpt7::CurrentMa.encode_to_bytes(0).unwrap(), [0x00, 0x00]);

        // 100 mA
        assert_eq!(Dpt7::CurrentMa.encode_to_bytes(100).unwrap(), [0x00, 0x64]);

        // 1000 mA = 1A
        assert_eq!(Dpt7::CurrentMa.encode_to_bytes(1000).unwrap(), [0x03, 0xE8]);

        // 65535 mA = 65.535A
        assert_eq!(Dpt7::CurrentMa.encode_to_bytes(65535).unwrap(), [0xFF, 0xFF]);
    }

    #[test]
    fn test_current_ma_decode() {
        assert_eq!(Dpt7::CurrentMa.decode(&[0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt7::CurrentMa.decode(&[0x00, 0x64]).unwrap(), 100);
        assert_eq!(Dpt7::CurrentMa.decode(&[0x03, 0xE8]).unwrap(), 1000);
        assert_eq!(Dpt7::CurrentMa.decode(&[0xFF, 0xFF]).unwrap(), 65535);
    }

    #[test]
    fn test_length_mm_encode() {
        // 0 mm
        assert_eq!(Dpt7::LengthMm.encode_to_bytes(0).unwrap(), [0x00, 0x00]);

        // 1 meter = 1000 mm
        assert_eq!(Dpt7::LengthMm.encode_to_bytes(1000).unwrap(), [0x03, 0xE8]);

        // 10 meters = 10000 mm
        assert_eq!(Dpt7::LengthMm.encode_to_bytes(10000).unwrap(), [0x27, 0x10]);

        // ~65 meters (max)
        assert_eq!(Dpt7::LengthMm.encode_to_bytes(65535).unwrap(), [0xFF, 0xFF]);
    }

    #[test]
    fn test_decode_invalid_length() {
        // Empty data
        let result = Dpt7::Pulses.decode(&[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));

        // Too short (1 byte)
        let result = Dpt7::Pulses.decode(&[0x42]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_decode_extra_bytes() {
        // Extra bytes are ignored (only first 2 used)
        assert_eq!(Dpt7::Pulses.decode(&[0x04, 0xD2, 0xFF]).unwrap(), 1234);
        assert_eq!(Dpt7::Pulses.decode(&[0x13, 0x88, 0x00, 0x00]).unwrap(), 5000);
    }

    #[test]
    fn test_round_trip() {
        let test_values = [0, 1, 100, 1234, 5000, 10000, 32767, 65535];

        for value in test_values {
            let encoded = Dpt7::Pulses.encode_to_bytes(value).unwrap();
            let decoded = Dpt7::Pulses.decode(&encoded).unwrap();
            assert_eq!(decoded, value, "Round-trip failed for {}", value);
        }
    }

    #[test]
    fn test_big_endian_byte_order() {
        // Verify big-endian encoding
        // 0x1234 should be [0x12, 0x34], not [0x34, 0x12]
        let encoded = Dpt7::Pulses.encode_to_bytes(0x1234).unwrap();
        assert_eq!(encoded[0], 0x12);
        assert_eq!(encoded[1], 0x34);

        // Verify decoding
        assert_eq!(Dpt7::Pulses.decode(&[0x12, 0x34]).unwrap(), 0x1234);
    }

    #[test]
    fn test_identifier() {
        assert_eq!(Dpt7::Pulses.identifier(), "7.001");
        assert_eq!(Dpt7::Brightness.identifier(), "7.013");
        assert_eq!(Dpt7::ColorTemperature.identifier(), "7.600");
    }

    #[test]
    fn test_unit() {
        assert_eq!(Dpt7::Pulses.unit(), "pulses");
        assert_eq!(Dpt7::Brightness.unit(), "lux");
        assert_eq!(Dpt7::ColorTemperature.unit(), "K");
        assert_eq!(Dpt7::TimePeriodSec.unit(), "s");
        assert_eq!(Dpt7::CurrentMa.unit(), "mA");
    }

    #[test]
    fn test_range() {
        assert_eq!(Dpt7::Pulses.range(), (0, 65535));
        assert_eq!(Dpt7::Brightness.range(), (0, 65535));
        assert_eq!(Dpt7::ColorTemperature.range(), (0, 65535));
    }
}
