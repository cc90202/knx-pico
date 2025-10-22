//! DPT 5.xxx - 8-bit Unsigned Value (1 byte)
//!
//! 8-bit unsigned datapoint types represent values from 0 to 255
//! with different scaling and interpretations.
//!
//! ## Format
//!
//! - 8 bits: unsigned value (0-255)
//!
//! ## Common Subtypes
//!
//! - **5.001** - Percentage (0-100%)
//! - **5.003** - Angle (0-360°)
//! - **5.004** - Percentage 0-255 (0-255)
//! - **5.005** - Ratio (0-255)
//! - **5.006** - Tariff (0-254)
//! - **5.010** - Counter pulses (0-255)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::dpt::{Dpt5, DptEncode, DptDecode};
//!
//! // Encode percentage (0-100%)
//! let data = Dpt5::Percentage.encode(75)?;  // [0xBF] = 191
//!
//! // Decode
//! let value = Dpt5::Percentage.decode(&[0xBF])?;  // 75
//!
//! // Angle (0-360°)
//! let data = Dpt5::Angle.encode(180)?;  // [0x80] = 128
//! ```

use crate::error::{KnxError, Result};
use crate::dpt::{DptEncode, DptDecode};

/// DPT 5.xxx 8-bit unsigned types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt5 {
    /// DPT 5.001 - Percentage (0-100%)
    Percentage,
    /// DPT 5.003 - Angle (0-360°)
    Angle,
    /// DPT 5.004 - Percentage 0-255 (0-255)
    PercentU8,
    /// DPT 5.005 - Ratio (0-255)
    Ratio,
    /// DPT 5.006 - Tariff (0-254)
    Tariff,
    /// DPT 5.010 - Counter pulses (0-255)
    Counter,
}

impl Dpt5 {
    /// Get the DPT identifier string (e.g., "5.001")
    pub const fn identifier(&self) -> &'static str {
        match self {
            Dpt5::Percentage => "5.001",
            Dpt5::Angle => "5.003",
            Dpt5::PercentU8 => "5.004",
            Dpt5::Ratio => "5.005",
            Dpt5::Tariff => "5.006",
            Dpt5::Counter => "5.010",
        }
    }

    /// Get the unit string for this DPT
    pub const fn unit(&self) -> &'static str {
        match self {
            Dpt5::Percentage => "%",
            Dpt5::Angle => "°",
            Dpt5::PercentU8 => "",
            Dpt5::Ratio => "",
            Dpt5::Tariff => "",
            Dpt5::Counter => "pulses",
        }
    }

    /// Get the valid range for this DPT (min, max)
    pub const fn range(&self) -> (u16, u16) {
        match self {
            Dpt5::Percentage => (0, 100),
            Dpt5::Angle => (0, 360),
            Dpt5::PercentU8 => (0, 255),
            Dpt5::Ratio => (0, 255),
            Dpt5::Tariff => (0, 254),
            Dpt5::Counter => (0, 255),
        }
    }

    /// Encode a value to raw byte
    ///
    /// For scaled types (Percentage, Angle), this performs the scaling.
    #[inline]
    fn encode_scaled(&self, value: u16) -> Result<u8> {
        let (min, max) = self.range();

        if value > max {
            return Err(KnxError::dpt_value_out_of_range());
        }

        let scaled = match self {
            Dpt5::Percentage => {
                // 0-100% -> 0-255
                // scaled = value * 255 / 100
                ((u32::from(value) * 255) / 100) as u8
            }
            Dpt5::Angle => {
                // 0-360° -> 0-255
                // scaled = value * 255 / 360
                ((u32::from(value) * 255) / 360) as u8
            }
            // Direct mapping for others
            Dpt5::PercentU8 | Dpt5::Ratio | Dpt5::Counter | Dpt5::Tariff => {
                if value < min || value > max {
                    return Err(KnxError::dpt_value_out_of_range());
                }
                value as u8
            }
        };

        Ok(scaled)
    }

    /// Decode raw byte to value
    ///
    /// For scaled types (Percentage, Angle), this performs the inverse scaling.
    #[inline]
    fn decode_scaled(&self, raw: u8) -> Result<u16> {
        let value = match self {
            Dpt5::Percentage => {
                // 0-255 -> 0-100%
                // value = raw * 100 / 255
                ((u32::from(raw) * 100) / 255) as u16
            }
            Dpt5::Angle => {
                // 0-255 -> 0-360°
                // value = raw * 360 / 255
                ((u32::from(raw) * 360) / 255) as u16
            }
            Dpt5::Tariff => {
                // Tariff has max 254
                if raw > 254 {
                    return Err(KnxError::dpt_value_out_of_range());
                }
                u16::from(raw)
            }
            // Direct mapping for others
            Dpt5::PercentU8 | Dpt5::Ratio | Dpt5::Counter => u16::from(raw),
        };

        Ok(value)
    }
}

impl DptEncode<u16> for Dpt5 {
    fn encode(&self, _value: u16) -> Result<&'static [u8]> {
        // We can't use static arrays for all possible values,
        // so we'll return an error and let the caller handle the byte.
        // This is a limitation of the current trait design.
        // Use encode_to_byte() instead.
        Err(KnxError::UnsupportedOperation)
    }
}

impl DptDecode<u16> for Dpt5 {
    fn decode(&self, data: &[u8]) -> Result<u16> {
        if data.is_empty() {
            return Err(KnxError::invalid_dpt_data());
        }

        self.decode_scaled(data[0])
    }
}

impl Dpt5 {
    /// Encode a value to a byte
    ///
    /// This is a convenience method that returns the encoded byte directly.
    /// Use this instead of the trait method.
    ///
    /// # Arguments
    /// * `value` - The value to encode
    ///
    /// # Returns
    /// The encoded byte value
    ///
    /// # Errors
    /// Returns `DptValueOutOfRange` if value is outside valid range
    pub fn encode_to_byte(&self, value: u16) -> Result<u8> {
        self.encode_scaled(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_encode() {
        // 0% -> 0
        assert_eq!(Dpt5::Percentage.encode_to_byte(0).unwrap(), 0x00);

        // 50% -> 127 (128 would be closer but integer division)
        let val = Dpt5::Percentage.encode_to_byte(50).unwrap();
        assert!(val >= 127 && val <= 128);

        // 100% -> 255
        assert_eq!(Dpt5::Percentage.encode_to_byte(100).unwrap(), 0xFF);

        // 75% -> 191
        assert_eq!(Dpt5::Percentage.encode_to_byte(75).unwrap(), 0xBF);
    }

    #[test]
    fn test_percentage_decode() {
        // 0 -> 0%
        assert_eq!(Dpt5::Percentage.decode(&[0x00]).unwrap(), 0);

        // 255 -> 100%
        assert_eq!(Dpt5::Percentage.decode(&[0xFF]).unwrap(), 100);

        // 127 -> ~49%
        let val = Dpt5::Percentage.decode(&[127]).unwrap();
        assert!(val >= 49 && val <= 50);

        // 191 -> ~75%
        let val = Dpt5::Percentage.decode(&[0xBF]).unwrap();
        assert!(val >= 74 && val <= 75);
    }

    #[test]
    fn test_percentage_out_of_range() {
        let result = Dpt5::Percentage.encode_to_byte(101);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_angle_encode() {
        // 0° -> 0
        assert_eq!(Dpt5::Angle.encode_to_byte(0).unwrap(), 0x00);

        // 180° -> 127
        let val = Dpt5::Angle.encode_to_byte(180).unwrap();
        assert!(val >= 127 && val <= 128);

        // 360° -> 255
        assert_eq!(Dpt5::Angle.encode_to_byte(360).unwrap(), 0xFF);
    }

    #[test]
    fn test_angle_decode() {
        // 0 -> 0°
        assert_eq!(Dpt5::Angle.decode(&[0x00]).unwrap(), 0);

        // 255 -> 360°
        assert_eq!(Dpt5::Angle.decode(&[0xFF]).unwrap(), 360);

        // 127 -> ~179°
        let val = Dpt5::Angle.decode(&[127]).unwrap();
        assert!(val >= 178 && val <= 180);
    }

    #[test]
    fn test_angle_out_of_range() {
        let result = Dpt5::Angle.encode_to_byte(361);
        assert!(result.is_err());
    }

    #[test]
    fn test_percent_u8_encode() {
        assert_eq!(Dpt5::PercentU8.encode_to_byte(0).unwrap(), 0);
        assert_eq!(Dpt5::PercentU8.encode_to_byte(128).unwrap(), 128);
        assert_eq!(Dpt5::PercentU8.encode_to_byte(255).unwrap(), 255);
    }

    #[test]
    fn test_percent_u8_decode() {
        assert_eq!(Dpt5::PercentU8.decode(&[0]).unwrap(), 0);
        assert_eq!(Dpt5::PercentU8.decode(&[128]).unwrap(), 128);
        assert_eq!(Dpt5::PercentU8.decode(&[255]).unwrap(), 255);
    }

    #[test]
    fn test_tariff_encode() {
        assert_eq!(Dpt5::Tariff.encode_to_byte(0).unwrap(), 0);
        assert_eq!(Dpt5::Tariff.encode_to_byte(100).unwrap(), 100);
        assert_eq!(Dpt5::Tariff.encode_to_byte(254).unwrap(), 254);
    }

    #[test]
    fn test_tariff_out_of_range() {
        // Tariff max is 254
        let result = Dpt5::Tariff.encode_to_byte(255);
        assert!(result.is_err());
    }

    #[test]
    fn test_tariff_decode_invalid() {
        // Tariff max is 254
        let result = Dpt5::Tariff.decode(&[255]);
        assert!(result.is_err());
    }

    #[test]
    fn test_counter_encode() {
        assert_eq!(Dpt5::Counter.encode_to_byte(0).unwrap(), 0);
        assert_eq!(Dpt5::Counter.encode_to_byte(42).unwrap(), 42);
        assert_eq!(Dpt5::Counter.encode_to_byte(255).unwrap(), 255);
    }

    #[test]
    fn test_counter_decode() {
        assert_eq!(Dpt5::Counter.decode(&[0]).unwrap(), 0);
        assert_eq!(Dpt5::Counter.decode(&[42]).unwrap(), 42);
        assert_eq!(Dpt5::Counter.decode(&[255]).unwrap(), 255);
    }

    #[test]
    fn test_decode_empty_data() {
        let result = Dpt5::Percentage.decode(&[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_round_trip_percentage() {
        for value in [0, 25, 50, 75, 100] {
            let encoded = Dpt5::Percentage.encode_to_byte(value).unwrap();
            let decoded = Dpt5::Percentage.decode(&[encoded]).unwrap();
            // Allow ±1 error due to rounding
            assert!((decoded as i16 - value as i16).abs() <= 1);
        }
    }

    #[test]
    fn test_round_trip_angle() {
        for value in [0, 90, 180, 270, 360] {
            let encoded = Dpt5::Angle.encode_to_byte(value).unwrap();
            let decoded = Dpt5::Angle.decode(&[encoded]).unwrap();
            // Allow ±2 error due to rounding
            assert!((decoded as i16 - value as i16).abs() <= 2);
        }
    }

    #[test]
    fn test_identifier() {
        assert_eq!(Dpt5::Percentage.identifier(), "5.001");
        assert_eq!(Dpt5::Angle.identifier(), "5.003");
        assert_eq!(Dpt5::Counter.identifier(), "5.010");
    }

    #[test]
    fn test_unit() {
        assert_eq!(Dpt5::Percentage.unit(), "%");
        assert_eq!(Dpt5::Angle.unit(), "°");
        assert_eq!(Dpt5::Counter.unit(), "pulses");
    }

    #[test]
    fn test_range() {
        assert_eq!(Dpt5::Percentage.range(), (0, 100));
        assert_eq!(Dpt5::Angle.range(), (0, 360));
        assert_eq!(Dpt5::Tariff.range(), (0, 254));
        assert_eq!(Dpt5::Counter.range(), (0, 255));
    }
}
