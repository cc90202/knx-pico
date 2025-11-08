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
//! let mut buf = [0u8; 1];
//!
//! // Encode percentage (0-100%)
//! let len = Dpt5::Percentage.encode(75, &mut buf)?;  // len = 1, buf = [0xBF] = 191
//!
//! // Decode
//! let value = Dpt5::Percentage.decode(&buf[..len])?;  // 75
//!
//! // Angle (0-360°)
//! let len = Dpt5::Angle.encode(180, &mut buf)?;  // len = 1, buf = [0x80] = 128
//! let angle = Dpt5::Angle.decode(&buf[..len])?;  // 180
//! ```

use crate::dpt::{DptDecode, DptEncode};
use crate::error::{KnxError, Result};

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
    fn encode(&self, value: u16, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Err(KnxError::buffer_too_small());
        }

        buf[0] = self.encode_scaled(value)?;
        Ok(1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_encode() {
        let mut buf = [0u8; 1];

        // 0% -> 0
        let len = Dpt5::Percentage.encode(0, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0x00);

        // 50% -> 127 (128 would be closer but integer division)
        let len = Dpt5::Percentage.encode(50, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert!(buf[0] >= 127 && buf[0] <= 128);

        // 100% -> 255
        let len = Dpt5::Percentage.encode(100, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0xFF);

        // 75% -> 191
        let len = Dpt5::Percentage.encode(75, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0xBF);
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
        let mut buf = [0u8; 1];
        let result = Dpt5::Percentage.encode(101, &mut buf);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_angle_encode() {
        let mut buf = [0u8; 1];

        // 0° -> 0
        let len = Dpt5::Angle.encode(0, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0x00);

        // 180° -> 127
        let len = Dpt5::Angle.encode(180, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert!(buf[0] >= 127 && buf[0] <= 128);

        // 360° -> 255
        let len = Dpt5::Angle.encode(360, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0xFF);
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
        let mut buf = [0u8; 1];
        let result = Dpt5::Angle.encode(361, &mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_percent_u8_encode() {
        let mut buf = [0u8; 1];

        let len = Dpt5::PercentU8.encode(0, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0);

        let len = Dpt5::PercentU8.encode(128, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 128);

        let len = Dpt5::PercentU8.encode(255, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 255);
    }

    #[test]
    fn test_percent_u8_decode() {
        assert_eq!(Dpt5::PercentU8.decode(&[0]).unwrap(), 0);
        assert_eq!(Dpt5::PercentU8.decode(&[128]).unwrap(), 128);
        assert_eq!(Dpt5::PercentU8.decode(&[255]).unwrap(), 255);
    }

    #[test]
    fn test_tariff_encode() {
        let mut buf = [0u8; 1];

        let len = Dpt5::Tariff.encode(0, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0);

        let len = Dpt5::Tariff.encode(100, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 100);

        let len = Dpt5::Tariff.encode(254, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 254);
    }

    #[test]
    fn test_tariff_out_of_range() {
        let mut buf = [0u8; 1];
        // Tariff max is 254
        let result = Dpt5::Tariff.encode(255, &mut buf);
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
        let mut buf = [0u8; 1];

        let len = Dpt5::Counter.encode(0, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 0);

        let len = Dpt5::Counter.encode(42, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 42);

        let len = Dpt5::Counter.encode(255, &mut buf).unwrap();
        assert_eq!(len, 1);
        assert_eq!(buf[0], 255);
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
        let mut buf = [0u8; 1];
        for value in [0, 25, 50, 75, 100] {
            let len = Dpt5::Percentage.encode(value, &mut buf).unwrap();
            assert_eq!(len, 1);
            let decoded = Dpt5::Percentage.decode(&buf[..len]).unwrap();
            // Allow ±1 error due to rounding
            assert!((decoded as i16 - value as i16).abs() <= 1);
        }
    }

    #[test]
    fn test_round_trip_angle() {
        let mut buf = [0u8; 1];
        for value in [0, 90, 180, 270, 360] {
            let len = Dpt5::Angle.encode(value, &mut buf).unwrap();
            assert_eq!(len, 1);
            let decoded = Dpt5::Angle.decode(&buf[..len]).unwrap();
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
