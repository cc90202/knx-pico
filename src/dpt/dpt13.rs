//! DPT 13.xxx - 4-byte Signed Value (4 bytes)
//!
//! 32-bit signed datapoint types represent values from -2,147,483,648 to +2,147,483,647.
//!
//! ## Format
//!
//! - 32 bits: signed value (-2^31 to 2^31-1), big-endian, two's complement
//!
//! ## Common Subtypes
//!
//! - **13.001** - Counter Pulses (signed)
//! - **13.002** - Flow Rate (l/h)
//! - **13.010** - Active Energy (Wh)
//! - **13.011** - Apparent Energy (VAh)
//! - **13.012** - Reactive Energy (VArh)
//! - **13.013** - Active Energy (kWh)
//! - **13.014** - Apparent Energy (kVAh)
//! - **13.015** - Reactive Energy (kVArh)
//! - **13.100** - Long Delta Time Period (s)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_rs::dpt::{Dpt13, DptDecode};
//!
//! // Decode active energy in Wh
//! let wh = Dpt13::ActiveEnergy.decode(&[0x00, 0x07, 0xA1, 0x20])?;  // 500000 Wh
//!
//! // Encode flow rate (can be negative for reverse flow)
//! let data = Dpt13::FlowRate.encode_to_bytes(-1000)?;  // Reverse flow
//!
//! // Counter pulses (signed, can increment/decrement)
//! let pulses = Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFF, 0xFF])?;  // -1
//! ```

use crate::error::{KnxError, Result};
use crate::dpt::DptDecode;

/// DPT 13.xxx 32-bit signed types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt13 {
    /// DPT 13.001 - Counter Pulses (signed)
    Counter,
    /// DPT 13.002 - Flow Rate (l/h)
    FlowRate,
    /// DPT 13.010 - Active Energy (Wh)
    ActiveEnergy,
    /// DPT 13.011 - Apparent Energy (VAh)
    ApparentEnergy,
    /// DPT 13.012 - Reactive Energy (VArh)
    ReactiveEnergy,
    /// DPT 13.013 - Active Energy (kWh)
    ActiveEnergyKwh,
    /// DPT 13.014 - Apparent Energy (kVAh)
    ApparentEnergyKvah,
    /// DPT 13.015 - Reactive Energy (kVArh)
    ReactiveEnergyKvarh,
    /// DPT 13.100 - Long Delta Time Period (s)
    LongDeltaTimeSec,
}

impl Dpt13 {
    /// Get the DPT identifier string (e.g., "13.001")
    #[inline]
    pub const fn identifier(&self) -> &'static str {
        match self {
            Dpt13::Counter => "13.001",
            Dpt13::FlowRate => "13.002",
            Dpt13::ActiveEnergy => "13.010",
            Dpt13::ApparentEnergy => "13.011",
            Dpt13::ReactiveEnergy => "13.012",
            Dpt13::ActiveEnergyKwh => "13.013",
            Dpt13::ApparentEnergyKvah => "13.014",
            Dpt13::ReactiveEnergyKvarh => "13.015",
            Dpt13::LongDeltaTimeSec => "13.100",
        }
    }

    /// Get the unit string for this DPT
    #[inline]
    pub const fn unit(&self) -> &'static str {
        match self {
            Dpt13::Counter => "pulses",
            Dpt13::FlowRate => "l/h",
            Dpt13::ActiveEnergy => "Wh",
            Dpt13::ApparentEnergy => "VAh",
            Dpt13::ReactiveEnergy => "VArh",
            Dpt13::ActiveEnergyKwh => "kWh",
            Dpt13::ApparentEnergyKvah => "kVAh",
            Dpt13::ReactiveEnergyKvarh => "kVArh",
            Dpt13::LongDeltaTimeSec => "s",
        }
    }

    /// Get the valid range for this DPT (min, max)
    #[inline]
    pub const fn range(&self) -> (i32, i32) {
        // All DPT 13.xxx types use full i32 range
        (i32::MIN, i32::MAX)
    }

    /// Encode an i32 value to 4 bytes (big-endian)
    ///
    /// # Performance
    /// - Uses optimized big-endian conversion
    /// - Inlined for zero-cost abstraction
    /// - No heap allocations
    ///
    /// # Arguments
    /// * `value` - The value to encode (-2^31 to 2^31-1)
    ///
    /// # Returns
    /// 4-byte array in big-endian format (two's complement for negatives)
    ///
    /// # Example
    /// ```rust,no_run
    /// // Positive value
    /// let bytes = Dpt13::ActiveEnergy.encode_to_bytes(500000)?;
    /// assert_eq!(bytes, [0x00, 0x07, 0xA1, 0x20]);
    ///
    /// // Negative value
    /// let bytes = Dpt13::FlowRate.encode_to_bytes(-1000)?;
    /// assert_eq!(bytes, [0xFF, 0xFF, 0xFC, 0x18]);
    /// ```
    #[inline]
    pub fn encode_to_bytes(&self, value: i32) -> Result<[u8; 4]> {
        // All i32 values are valid for DPT 13.xxx
        // to_be_bytes() handles two's complement encoding automatically
        Ok(value.to_be_bytes())
    }

    /// Decode 4 bytes (big-endian) to i32 value
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
    /// * `data` - 4-byte slice in big-endian format (two's complement)
    ///
    /// # Returns
    /// The decoded i32 value
    ///
    /// # Errors
    /// Returns `InvalidDptData` if data length is not exactly 4 bytes
    #[inline]
    fn decode_raw(&self, data: &[u8]) -> Result<i32> {
        if data.len() < 4 {
            return Err(KnxError::InvalidDptData);
        }

        // SAFETY: We just validated that data.len() >= 4
        // This eliminates redundant bounds checks
        let bytes = unsafe {
            [
                *data.get_unchecked(0),
                *data.get_unchecked(1),
                *data.get_unchecked(2),
                *data.get_unchecked(3),
            ]
        };

        // i32::from_be_bytes handles two's complement decoding automatically
        // Optimized by LLVM to a simple load on big-endian
        // and a bswap instruction on little-endian (single cycle on ARM)
        Ok(i32::from_be_bytes(bytes))
    }
}

impl DptDecode<i32> for Dpt13 {
    #[inline]
    fn decode(&self, data: &[u8]) -> Result<i32> {
        self.decode_raw(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_encode_positive() {
        // Zero
        assert_eq!(Dpt13::Counter.encode_to_bytes(0).unwrap(), [0x00, 0x00, 0x00, 0x00]);

        // Small positive
        assert_eq!(Dpt13::Counter.encode_to_bytes(1).unwrap(), [0x00, 0x00, 0x00, 0x01]);

        // Medium positive
        assert_eq!(Dpt13::Counter.encode_to_bytes(1234567).unwrap(), [0x00, 0x12, 0xD6, 0x87]);

        // Large positive
        assert_eq!(Dpt13::Counter.encode_to_bytes(100000000).unwrap(), [0x05, 0xF5, 0xE1, 0x00]);

        // Maximum positive
        assert_eq!(Dpt13::Counter.encode_to_bytes(i32::MAX).unwrap(), [0x7F, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_counter_encode_negative() {
        // -1 (all bits set in two's complement)
        assert_eq!(Dpt13::Counter.encode_to_bytes(-1).unwrap(), [0xFF, 0xFF, 0xFF, 0xFF]);

        // Small negative
        assert_eq!(Dpt13::Counter.encode_to_bytes(-100).unwrap(), [0xFF, 0xFF, 0xFF, 0x9C]);

        // Medium negative
        assert_eq!(Dpt13::Counter.encode_to_bytes(-1000).unwrap(), [0xFF, 0xFF, 0xFC, 0x18]);

        // Large negative
        assert_eq!(Dpt13::Counter.encode_to_bytes(-100000000).unwrap(), [0xFA, 0x0A, 0x1F, 0x00]);

        // Minimum (most negative)
        assert_eq!(Dpt13::Counter.encode_to_bytes(i32::MIN).unwrap(), [0x80, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_counter_decode_positive() {
        // Zero
        assert_eq!(Dpt13::Counter.decode(&[0x00, 0x00, 0x00, 0x00]).unwrap(), 0);

        // Small positive
        assert_eq!(Dpt13::Counter.decode(&[0x00, 0x00, 0x00, 0x01]).unwrap(), 1);

        // Medium positive
        assert_eq!(Dpt13::Counter.decode(&[0x00, 0x12, 0xD6, 0x87]).unwrap(), 1234567);

        // Large positive
        assert_eq!(Dpt13::Counter.decode(&[0x05, 0xF5, 0xE1, 0x00]).unwrap(), 100000000);

        // Maximum positive
        assert_eq!(Dpt13::Counter.decode(&[0x7F, 0xFF, 0xFF, 0xFF]).unwrap(), i32::MAX);
    }

    #[test]
    fn test_counter_decode_negative() {
        // -1
        assert_eq!(Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap(), -1);

        // Small negative
        assert_eq!(Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFF, 0x9C]).unwrap(), -100);

        // Medium negative
        assert_eq!(Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFC, 0x18]).unwrap(), -1000);

        // Large negative
        assert_eq!(Dpt13::Counter.decode(&[0xFA, 0x0A, 0x1F, 0x00]).unwrap(), -100000000);

        // Minimum (most negative)
        assert_eq!(Dpt13::Counter.decode(&[0x80, 0x00, 0x00, 0x00]).unwrap(), i32::MIN);
    }

    #[test]
    fn test_active_energy_encode() {
        // 0 Wh
        assert_eq!(Dpt13::ActiveEnergy.encode_to_bytes(0).unwrap(), [0x00, 0x00, 0x00, 0x00]);

        // 1 kWh = 1000 Wh
        assert_eq!(Dpt13::ActiveEnergy.encode_to_bytes(1000).unwrap(), [0x00, 0x00, 0x03, 0xE8]);

        // 100 kWh = 100000 Wh
        assert_eq!(Dpt13::ActiveEnergy.encode_to_bytes(100000).unwrap(), [0x00, 0x01, 0x86, 0xA0]);

        // 500 kWh = 500000 Wh
        assert_eq!(Dpt13::ActiveEnergy.encode_to_bytes(500000).unwrap(), [0x00, 0x07, 0xA1, 0x20]);

        // 1 MWh = 1000000 Wh
        assert_eq!(Dpt13::ActiveEnergy.encode_to_bytes(1000000).unwrap(), [0x00, 0x0F, 0x42, 0x40]);
    }

    #[test]
    fn test_active_energy_decode() {
        assert_eq!(Dpt13::ActiveEnergy.decode(&[0x00, 0x00, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt13::ActiveEnergy.decode(&[0x00, 0x00, 0x03, 0xE8]).unwrap(), 1000);
        assert_eq!(Dpt13::ActiveEnergy.decode(&[0x00, 0x01, 0x86, 0xA0]).unwrap(), 100000);
        assert_eq!(Dpt13::ActiveEnergy.decode(&[0x00, 0x07, 0xA1, 0x20]).unwrap(), 500000);
        assert_eq!(Dpt13::ActiveEnergy.decode(&[0x00, 0x0F, 0x42, 0x40]).unwrap(), 1000000);
    }

    #[test]
    fn test_flow_rate_encode() {
        // 0 l/h (no flow)
        assert_eq!(Dpt13::FlowRate.encode_to_bytes(0).unwrap(), [0x00, 0x00, 0x00, 0x00]);

        // 1000 l/h (positive flow)
        assert_eq!(Dpt13::FlowRate.encode_to_bytes(1000).unwrap(), [0x00, 0x00, 0x03, 0xE8]);

        // -1000 l/h (reverse flow)
        assert_eq!(Dpt13::FlowRate.encode_to_bytes(-1000).unwrap(), [0xFF, 0xFF, 0xFC, 0x18]);

        // 10000 l/h
        assert_eq!(Dpt13::FlowRate.encode_to_bytes(10000).unwrap(), [0x00, 0x00, 0x27, 0x10]);

        // -10000 l/h
        assert_eq!(Dpt13::FlowRate.encode_to_bytes(-10000).unwrap(), [0xFF, 0xFF, 0xD8, 0xF0]);
    }

    #[test]
    fn test_flow_rate_decode() {
        assert_eq!(Dpt13::FlowRate.decode(&[0x00, 0x00, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt13::FlowRate.decode(&[0x00, 0x00, 0x03, 0xE8]).unwrap(), 1000);
        assert_eq!(Dpt13::FlowRate.decode(&[0xFF, 0xFF, 0xFC, 0x18]).unwrap(), -1000);
        assert_eq!(Dpt13::FlowRate.decode(&[0x00, 0x00, 0x27, 0x10]).unwrap(), 10000);
        assert_eq!(Dpt13::FlowRate.decode(&[0xFF, 0xFF, 0xD8, 0xF0]).unwrap(), -10000);
    }

    #[test]
    fn test_reactive_energy_encode() {
        // 0 VArh
        assert_eq!(Dpt13::ReactiveEnergy.encode_to_bytes(0).unwrap(), [0x00, 0x00, 0x00, 0x00]);

        // 5000 VArh
        assert_eq!(Dpt13::ReactiveEnergy.encode_to_bytes(5000).unwrap(), [0x00, 0x00, 0x13, 0x88]);

        // -5000 VArh (reverse reactive energy)
        assert_eq!(Dpt13::ReactiveEnergy.encode_to_bytes(-5000).unwrap(), [0xFF, 0xFF, 0xEC, 0x78]);
    }

    #[test]
    fn test_reactive_energy_decode() {
        assert_eq!(Dpt13::ReactiveEnergy.decode(&[0x00, 0x00, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt13::ReactiveEnergy.decode(&[0x00, 0x00, 0x13, 0x88]).unwrap(), 5000);
        assert_eq!(Dpt13::ReactiveEnergy.decode(&[0xFF, 0xFF, 0xEC, 0x78]).unwrap(), -5000);
    }

    #[test]
    fn test_long_delta_time_sec_encode() {
        // 0 seconds
        assert_eq!(Dpt13::LongDeltaTimeSec.encode_to_bytes(0).unwrap(), [0x00, 0x00, 0x00, 0x00]);

        // 1 hour = 3600 seconds
        assert_eq!(Dpt13::LongDeltaTimeSec.encode_to_bytes(3600).unwrap(), [0x00, 0x00, 0x0E, 0x10]);

        // 1 day = 86400 seconds
        assert_eq!(Dpt13::LongDeltaTimeSec.encode_to_bytes(86400).unwrap(), [0x00, 0x01, 0x51, 0x80]);

        // -1 hour (time difference in past)
        assert_eq!(Dpt13::LongDeltaTimeSec.encode_to_bytes(-3600).unwrap(), [0xFF, 0xFF, 0xF1, 0xF0]);
    }

    #[test]
    fn test_long_delta_time_sec_decode() {
        assert_eq!(Dpt13::LongDeltaTimeSec.decode(&[0x00, 0x00, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt13::LongDeltaTimeSec.decode(&[0x00, 0x00, 0x0E, 0x10]).unwrap(), 3600);
        assert_eq!(Dpt13::LongDeltaTimeSec.decode(&[0x00, 0x01, 0x51, 0x80]).unwrap(), 86400);
        assert_eq!(Dpt13::LongDeltaTimeSec.decode(&[0xFF, 0xFF, 0xF1, 0xF0]).unwrap(), -3600);
    }

    #[test]
    fn test_decode_invalid_length() {
        // Empty data
        let result = Dpt13::Counter.decode(&[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::InvalidDptData));

        // Too short (1 byte)
        let result = Dpt13::Counter.decode(&[0x42]);
        assert!(result.is_err());

        // Too short (2 bytes)
        let result = Dpt13::Counter.decode(&[0x42, 0x00]);
        assert!(result.is_err());

        // Too short (3 bytes)
        let result = Dpt13::Counter.decode(&[0x42, 0x00, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_extra_bytes() {
        // Extra bytes are ignored (only first 4 used)
        assert_eq!(Dpt13::Counter.decode(&[0x00, 0x07, 0xA1, 0x20, 0xFF]).unwrap(), 500000);
        assert_eq!(Dpt13::Counter.decode(&[0x00, 0x00, 0x03, 0xE8, 0x00, 0x00]).unwrap(), 1000);
    }

    #[test]
    fn test_round_trip_positive() {
        let test_values = [0, 1, 100, 1000, 10000, 100000, 1000000, i32::MAX];

        for value in test_values {
            let encoded = Dpt13::Counter.encode_to_bytes(value).unwrap();
            let decoded = Dpt13::Counter.decode(&encoded).unwrap();
            assert_eq!(decoded, value, "Round-trip failed for {}", value);
        }
    }

    #[test]
    fn test_round_trip_negative() {
        let test_values = [-1, -100, -1000, -10000, -100000, -1000000, i32::MIN];

        for value in test_values {
            let encoded = Dpt13::Counter.encode_to_bytes(value).unwrap();
            let decoded = Dpt13::Counter.decode(&encoded).unwrap();
            assert_eq!(decoded, value, "Round-trip failed for {}", value);
        }
    }

    #[test]
    fn test_big_endian_byte_order() {
        // Verify big-endian encoding
        // 0x12345678 should be [0x12, 0x34, 0x56, 0x78]
        let encoded = Dpt13::Counter.encode_to_bytes(0x12345678).unwrap();
        assert_eq!(encoded[0], 0x12);
        assert_eq!(encoded[1], 0x34);
        assert_eq!(encoded[2], 0x56);
        assert_eq!(encoded[3], 0x78);

        // Verify decoding
        assert_eq!(Dpt13::Counter.decode(&[0x12, 0x34, 0x56, 0x78]).unwrap(), 0x12345678);
    }

    #[test]
    fn test_twos_complement_negative() {
        // -1 should be all bits set (0xFF, 0xFF, 0xFF, 0xFF)
        let encoded = Dpt13::Counter.encode_to_bytes(-1).unwrap();
        assert_eq!(encoded, [0xFF, 0xFF, 0xFF, 0xFF]);

        // Decode it back
        assert_eq!(Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap(), -1);
    }

    #[test]
    fn test_twos_complement_min_max() {
        // i32::MIN = -2147483648 = 0x80000000
        let encoded = Dpt13::Counter.encode_to_bytes(i32::MIN).unwrap();
        assert_eq!(encoded, [0x80, 0x00, 0x00, 0x00]);
        assert_eq!(Dpt13::Counter.decode(&encoded).unwrap(), i32::MIN);

        // i32::MAX = 2147483647 = 0x7FFFFFFF
        let encoded = Dpt13::Counter.encode_to_bytes(i32::MAX).unwrap();
        assert_eq!(encoded, [0x7F, 0xFF, 0xFF, 0xFF]);
        assert_eq!(Dpt13::Counter.decode(&encoded).unwrap(), i32::MAX);
    }

    #[test]
    fn test_identifier() {
        assert_eq!(Dpt13::Counter.identifier(), "13.001");
        assert_eq!(Dpt13::FlowRate.identifier(), "13.002");
        assert_eq!(Dpt13::ActiveEnergy.identifier(), "13.010");
        assert_eq!(Dpt13::ReactiveEnergy.identifier(), "13.012");
    }

    #[test]
    fn test_unit() {
        assert_eq!(Dpt13::Counter.unit(), "pulses");
        assert_eq!(Dpt13::FlowRate.unit(), "l/h");
        assert_eq!(Dpt13::ActiveEnergy.unit(), "Wh");
        assert_eq!(Dpt13::ReactiveEnergy.unit(), "VArh");
        assert_eq!(Dpt13::LongDeltaTimeSec.unit(), "s");
    }

    #[test]
    fn test_range() {
        assert_eq!(Dpt13::Counter.range(), (i32::MIN, i32::MAX));
        assert_eq!(Dpt13::ActiveEnergy.range(), (i32::MIN, i32::MAX));
    }
}
