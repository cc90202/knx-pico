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
//! - **13.011** - Apparent Energy (`VAh`)
//! - **13.012** - Reactive Energy (`VArh`)
//! - **13.013** - Active Energy (kWh)
//! - **13.014** - Apparent Energy (kVAh)
//! - **13.015** - Reactive Energy (kVArh)
//! - **13.100** - Long Delta Time Period (s)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::dpt::{Dpt13, DptDecode, DptEncode};
//!
//! // Decode active energy in Wh
//! let wh = Dpt13::ActiveEnergy.decode(&[0x00, 0x07, 0xA1, 0x20])?;  // 500000 Wh
//!
//! // Encode flow rate using trait (can be negative for reverse flow)
//! let mut buf = [0u8; 4];
//! let len = Dpt13::FlowRate.encode(-1000, &mut buf)?;  // Reverse flow
//!
//! // Counter pulses (signed, can increment/decrement)
//! let pulses = Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFF, 0xFF])?;  // -1
//! ```

use crate::error::{KnxError, Result};
use crate::dpt::{DptDecode, DptEncode};

/// DPT 13.xxx 32-bit signed types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt13 {
    /// DPT 13.001 - Counter Pulses (signed)
    Counter,
    /// DPT 13.002 - Flow Rate (l/h)
    FlowRate,
    /// DPT 13.010 - Active Energy (Wh)
    ActiveEnergy,
    /// DPT 13.011 - Apparent Energy (`VAh`)
    ApparentEnergy,
    /// DPT 13.012 - Reactive Energy (`VArh`)
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
            return Err(KnxError::invalid_dpt_data());
        }

        // SAFETY: Bounds checked above - data.len() >= 4 is guaranteed.
        // Indices 0, 1, 2, and 3 are all < 4, therefore within bounds.
        // Using get_unchecked eliminates redundant bounds checks in DPT decoding,
        // providing performance benefit for frequent value conversions in KNX communication.
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

impl DptEncode<i32> for Dpt13 {
    fn encode(&self, value: i32, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 4 {
            return Err(KnxError::buffer_too_small());
        }

        let bytes = value.to_be_bytes();
        buf[0] = bytes[0];
        buf[1] = bytes[1];
        buf[2] = bytes[2];
        buf[3] = bytes[3];
        Ok(4)
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
        let mut buf = [0u8; 4];

        // Zero
        let len = Dpt13::Counter.encode(0, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x00, 0x00]);

        // Small positive
        let len = Dpt13::Counter.encode(1, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x00, 0x01]);

        // Medium positive
        let len = Dpt13::Counter.encode(1234567, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x12, 0xD6, 0x87]);

        // Large positive
        let len = Dpt13::Counter.encode(100000000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x05, 0xF5, 0xE1, 0x00]);

        // Maximum positive
        let len = Dpt13::Counter.encode(i32::MAX, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x7F, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_counter_encode_negative() {
        let mut buf = [0u8; 4];

        // -1 (all bits set in two's complement)
        let len = Dpt13::Counter.encode(-1, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xFF, 0xFF]);

        // Small negative
        let len = Dpt13::Counter.encode(-100, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xFF, 0x9C]);

        // Medium negative
        let len = Dpt13::Counter.encode(-1000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xFC, 0x18]);

        // Large negative
        let len = Dpt13::Counter.encode(-100000000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFA, 0x0A, 0x1F, 0x00]);

        // Minimum (most negative)
        let len = Dpt13::Counter.encode(i32::MIN, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x80, 0x00, 0x00, 0x00]);
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
        let mut buf = [0u8; 4];

        // 0 Wh
        let len = Dpt13::ActiveEnergy.encode(0, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x00, 0x00]);

        // 1 kWh = 1000 Wh
        let len = Dpt13::ActiveEnergy.encode(1000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x03, 0xE8]);

        // 100 kWh = 100000 Wh
        let len = Dpt13::ActiveEnergy.encode(100000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x01, 0x86, 0xA0]);

        // 500 kWh = 500000 Wh
        let len = Dpt13::ActiveEnergy.encode(500000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x07, 0xA1, 0x20]);

        // 1 MWh = 1000000 Wh
        let len = Dpt13::ActiveEnergy.encode(1000000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x0F, 0x42, 0x40]);
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
        let mut buf = [0u8; 4];

        // 0 l/h (no flow)
        let len = Dpt13::FlowRate.encode(0, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x00, 0x00]);

        // 1000 l/h (positive flow)
        let len = Dpt13::FlowRate.encode(1000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x03, 0xE8]);

        // -1000 l/h (reverse flow)
        let len = Dpt13::FlowRate.encode(-1000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xFC, 0x18]);

        // 10000 l/h
        let len = Dpt13::FlowRate.encode(10000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x27, 0x10]);

        // -10000 l/h
        let len = Dpt13::FlowRate.encode(-10000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xD8, 0xF0]);
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
        let mut buf = [0u8; 4];

        // 0 VArh
        let len = Dpt13::ReactiveEnergy.encode(0, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x00, 0x00]);

        // 5000 VArh
        let len = Dpt13::ReactiveEnergy.encode(5000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x13, 0x88]);

        // -5000 VArh (reverse reactive energy)
        let len = Dpt13::ReactiveEnergy.encode(-5000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xEC, 0x78]);
    }

    #[test]
    fn test_reactive_energy_decode() {
        assert_eq!(Dpt13::ReactiveEnergy.decode(&[0x00, 0x00, 0x00, 0x00]).unwrap(), 0);
        assert_eq!(Dpt13::ReactiveEnergy.decode(&[0x00, 0x00, 0x13, 0x88]).unwrap(), 5000);
        assert_eq!(Dpt13::ReactiveEnergy.decode(&[0xFF, 0xFF, 0xEC, 0x78]).unwrap(), -5000);
    }

    #[test]
    fn test_long_delta_time_sec_encode() {
        let mut buf = [0u8; 4];

        // 0 seconds
        let len = Dpt13::LongDeltaTimeSec.encode(0, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x00, 0x00]);

        // 1 hour = 3600 seconds
        let len = Dpt13::LongDeltaTimeSec.encode(3600, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x00, 0x0E, 0x10]);

        // 1 day = 86400 seconds
        let len = Dpt13::LongDeltaTimeSec.encode(86400, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x00, 0x01, 0x51, 0x80]);

        // -1 hour (time difference in past)
        let len = Dpt13::LongDeltaTimeSec.encode(-3600, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xF1, 0xF0]);
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
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));

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
        let mut buf = [0u8; 4];
        let test_values = [0, 1, 100, 1000, 10000, 100000, 1000000, i32::MAX];

        for value in test_values {
            let len = Dpt13::Counter.encode(value, &mut buf).unwrap();
            assert_eq!(len, 4);
            let decoded = Dpt13::Counter.decode(&buf[..len]).unwrap();
            assert_eq!(decoded, value, "Round-trip failed for {}", value);
        }
    }

    #[test]
    fn test_round_trip_negative() {
        let mut buf = [0u8; 4];
        let test_values = [-1, -100, -1000, -10000, -100000, -1000000, i32::MIN];

        for value in test_values {
            let len = Dpt13::Counter.encode(value, &mut buf).unwrap();
            assert_eq!(len, 4);
            let decoded = Dpt13::Counter.decode(&buf[..len]).unwrap();
            assert_eq!(decoded, value, "Round-trip failed for {}", value);
        }
    }

    #[test]
    fn test_big_endian_byte_order() {
        let mut buf = [0u8; 4];

        // Verify big-endian encoding
        // 0x12345678 should be [0x12, 0x34, 0x56, 0x78]
        let len = Dpt13::Counter.encode(0x12345678, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf[0], 0x12);
        assert_eq!(buf[1], 0x34);
        assert_eq!(buf[2], 0x56);
        assert_eq!(buf[3], 0x78);

        // Verify decoding
        assert_eq!(Dpt13::Counter.decode(&[0x12, 0x34, 0x56, 0x78]).unwrap(), 0x12345678);
    }

    #[test]
    fn test_twos_complement_negative() {
        let mut buf = [0u8; 4];

        // -1 should be all bits set (0xFF, 0xFF, 0xFF, 0xFF)
        let len = Dpt13::Counter.encode(-1, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0xFF, 0xFF, 0xFF, 0xFF]);

        // Decode it back
        assert_eq!(Dpt13::Counter.decode(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap(), -1);
    }

    #[test]
    fn test_twos_complement_min_max() {
        let mut buf = [0u8; 4];

        // i32::MIN = -2147483648 = 0x80000000
        let len = Dpt13::Counter.encode(i32::MIN, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x80, 0x00, 0x00, 0x00]);
        assert_eq!(Dpt13::Counter.decode(&buf[..len]).unwrap(), i32::MIN);

        // i32::MAX = 2147483647 = 0x7FFFFFFF
        let len = Dpt13::Counter.encode(i32::MAX, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x7F, 0xFF, 0xFF, 0xFF]);
        assert_eq!(Dpt13::Counter.decode(&buf[..len]).unwrap(), i32::MAX);
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

    // =========================================================================
    // DptEncode Trait Tests
    // =========================================================================

    #[test]
    fn test_trait_encode_basic() {
        let mut buf = [0u8; 4];

        let len = Dpt13::Counter.encode(1234567, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf, [0x00, 0x12, 0xD6, 0x87]);

        let len = Dpt13::ActiveEnergy.encode(500000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf, [0x00, 0x07, 0xA1, 0x20]);
    }

    #[test]
    fn test_trait_encode_negative() {
        let mut buf = [0u8; 4];

        let len = Dpt13::Counter.encode(-1, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf, [0xFF, 0xFF, 0xFF, 0xFF]);

        let len = Dpt13::FlowRate.encode(-1000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf, [0xFF, 0xFF, 0xFC, 0x18]);
    }

    #[test]
    fn test_trait_encode_buffer_too_small() {
        let mut buf = [0u8; 3];
        let result = Dpt13::Counter.encode(1234, &mut buf);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Transport(_)));

        let mut buf = [0u8; 0];
        let result = Dpt13::Counter.encode(1234, &mut buf);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Transport(_)));
    }

    #[test]
    fn test_trait_encode_round_trip_positive() {
        let mut buf = [0u8; 4];
        let test_values = [0, 1, 100, 1000, 10000, 100000, 1000000, i32::MAX];

        for value in test_values {
            let len = Dpt13::Counter.encode(value, &mut buf).unwrap();
            assert_eq!(len, 4);

            let decoded = Dpt13::Counter.decode(&buf[..len]).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn test_trait_encode_round_trip_negative() {
        let mut buf = [0u8; 4];
        let test_values = [-1, -100, -1000, -10000, -100000, -1000000, i32::MIN];

        for value in test_values {
            let len = Dpt13::Counter.encode(value, &mut buf).unwrap();
            assert_eq!(len, 4);

            let decoded = Dpt13::Counter.decode(&buf[..len]).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn test_trait_encode_big_endian() {
        let mut buf = [0u8; 4];

        let len = Dpt13::Counter.encode(0x12345678, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf[0], 0x12);
        assert_eq!(buf[1], 0x34);
        assert_eq!(buf[2], 0x56);
        assert_eq!(buf[3], 0x78);
    }

    #[test]
    fn test_trait_encode_matches_big_endian() {
        let mut buf = [0u8; 4];
        let test_values = [0, 1000, -1000, 500000, -100000, i32::MIN, i32::MAX];

        for value in test_values {
            let len = Dpt13::ActiveEnergy.encode(value, &mut buf).unwrap();
            assert_eq!(len, 4);

            // Verify the bytes match expected big-endian encoding
            let expected = value.to_be_bytes();
            assert_eq!(&buf[..4], &expected);
        }
    }

    #[test]
    fn test_trait_encode_energy_values() {
        let mut buf = [0u8; 4];

        // 1 kWh = 1000 Wh
        let len = Dpt13::ActiveEnergy.encode(1000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf, [0x00, 0x00, 0x03, 0xE8]);

        // 1 MWh = 1000000 Wh
        let len = Dpt13::ActiveEnergy.encode(1000000, &mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(buf, [0x00, 0x0F, 0x42, 0x40]);
    }
}
