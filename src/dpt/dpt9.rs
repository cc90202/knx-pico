//! DPT 9.xxx - 2-byte Float (16-bit floating point)
//!
//! 2-byte floating point datapoint types represent values using a custom
//! 16-bit floating point format with 1 sign bit, 4 exponent bits, and 11 mantissa bits.
//!
//! ## Format
//!
//! ```text
//! Byte 0: SEEE EMMM
//! Byte 1: MMMM MMMM
//!
//! S = Sign bit (bit 15: 0 = positive, 1 = negative)
//! E = Exponent (bits 14-11: 4 bits, unsigned, range 0-15)
//! M = Mantissa (bits 10-0: 11 bits, unsigned magnitude)
//!
//! Value = (0.01 * M) * 2^E  (negated if S=1)
//! ```
//!
//! ## Range
//!
//! - Min: -671088.64
//! - Max: +670760.96
//! - Resolution: 0.01 at exponent 0
//!
//! ## Common Subtypes
//!
//! - **9.001** - Temperature (°C)
//! - **9.004** - Illuminance (lux)
//! - **9.005** - Wind speed (m/s)
//! - **9.006** - Pressure (Pa)
//! - **9.007** - Humidity (%)
//! - **9.008** - Air quality (ppm)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::dpt::{Dpt9, DptEncode, DptDecode};
//!
//! // Encode temperature using trait
//! let mut buf = [0u8; 2];
//! let len = Dpt9::Temperature.encode(21.5, &mut buf)?;
//! // buf[..len] contains encoded value
//!
//! // Decode
//! let temp = Dpt9::Temperature.decode(&buf)?;
//! // temp ≈ 21.5
//! ```

use crate::dpt::{DptDecode, DptEncode};
use crate::error::{KnxError, Result};

/// DPT 9.xxx 2-byte float types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt9 {
    /// DPT 9.001 - Temperature (°C)
    Temperature,
    /// DPT 9.002 - Temperature difference (K)
    TemperatureDifference,
    /// DPT 9.003 - Kelvin per hour (K/h)
    KelvinPerHour,
    /// DPT 9.004 - Illuminance (lux)
    Illuminance,
    /// DPT 9.005 - Wind speed (m/s)
    WindSpeed,
    /// DPT 9.006 - Pressure (Pa)
    Pressure,
    /// DPT 9.007 - Humidity (%)
    Humidity,
    /// DPT 9.008 - Air quality (ppm)
    AirQuality,
    /// DPT 9.010 - Time difference (s)
    TimeDifference,
    /// DPT 9.011 - Time difference (ms)
    TimeDifferenceMs,
    /// DPT 9.020 - Voltage (mV)
    Voltage,
    /// DPT 9.021 - Current (mA)
    Current,
    /// DPT 9.022 - Power density (W/m²)
    PowerDensity,
    /// DPT 9.023 - Kelvin per percent (K/%)
    KelvinPerPercent,
    /// DPT 9.024 - Power (kW)
    Power,
}

impl Dpt9 {
    /// Get the DPT identifier string
    pub const fn identifier(&self) -> &'static str {
        match self {
            Dpt9::Temperature => "9.001",
            Dpt9::TemperatureDifference => "9.002",
            Dpt9::KelvinPerHour => "9.003",
            Dpt9::Illuminance => "9.004",
            Dpt9::WindSpeed => "9.005",
            Dpt9::Pressure => "9.006",
            Dpt9::Humidity => "9.007",
            Dpt9::AirQuality => "9.008",
            Dpt9::TimeDifference => "9.010",
            Dpt9::TimeDifferenceMs => "9.011",
            Dpt9::Voltage => "9.020",
            Dpt9::Current => "9.021",
            Dpt9::PowerDensity => "9.022",
            Dpt9::KelvinPerPercent => "9.023",
            Dpt9::Power => "9.024",
        }
    }

    /// Get the unit string
    pub const fn unit(&self) -> &'static str {
        match self {
            Dpt9::Temperature => "°C",
            Dpt9::TemperatureDifference => "K",
            Dpt9::KelvinPerHour => "K/h",
            Dpt9::Illuminance => "lux",
            Dpt9::WindSpeed => "m/s",
            Dpt9::Pressure => "Pa",
            Dpt9::Humidity => "%",
            Dpt9::AirQuality => "ppm",
            Dpt9::TimeDifference => "s",
            Dpt9::TimeDifferenceMs => "ms",
            Dpt9::Voltage => "mV",
            Dpt9::Current => "mA",
            Dpt9::PowerDensity => "W/m²",
            Dpt9::KelvinPerPercent => "K/%",
            Dpt9::Power => "kW",
        }
    }

    /// Decode 2-byte KNX float format to f32
    ///
    /// # Arguments
    /// * `bytes` - The 2-byte array to decode
    ///
    /// # Returns
    /// The decoded floating point value
    pub fn decode_from_bytes(&self, bytes: &[u8]) -> Result<f32> {
        if bytes.len() < 2 {
            return Err(KnxError::invalid_dpt_data());
        }

        let value_u16 = u16::from_be_bytes([bytes[0], bytes[1]]);

        // Extract fields: SEEE EMMM MMMM MMMM
        // S = Sign bit (bit 15)
        // E = Exponent (bits 14-11, unsigned)
        // M = Mantissa (bits 10-0, two's complement when S=1)
        let sign_bit = (value_u16 >> 15) & 0x01;
        let exponent = ((value_u16 >> 11) & 0x0F) as u8;
        let mantissa_raw = value_u16 & 0x07FF;

        // Apply sign to mantissa
        // When sign=1, mantissa uses two's complement on 11 bits (subtract 2048)
        let mantissa = if sign_bit == 1 {
            (mantissa_raw as i16) - 2048
        } else {
            mantissa_raw as i16
        };

        // Calculate value = (0.01 * mantissa) * 2^exponent
        let value = (0.01 * f32::from(mantissa)) * (1u32 << exponent) as f32;

        Ok(value)
    }
}

impl DptEncode<f32> for Dpt9 {
    fn encode(&self, value: f32, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 2 {
            return Err(KnxError::buffer_too_small());
        }

        // Handle special case of zero
        if value == 0.0 {
            buf[0] = 0x00;
            buf[1] = 0x00;
            return Ok(2);
        }

        // Find the appropriate exponent (0-15)
        let mut exponent = 0u8;
        let mut mantissa_f = value * 100.0;

        // Scale to fit mantissa in 11-bit signed range: -1024 to +1023
        while !(-1024.0..=1023.0).contains(&mantissa_f) && exponent < 15 {
            exponent += 1;
            mantissa_f = value * 100.0 / (1u32 << exponent) as f32;
        }

        // Check range
        if !(-1024.0..=1023.0).contains(&mantissa_f) {
            return Err(KnxError::dpt_value_out_of_range());
        }

        // Round to nearest integer (manual rounding for no_std)
        let mantissa = if mantissa_f >= 0.0 {
            (mantissa_f + 0.5) as i16
        } else {
            (mantissa_f - 0.5) as i16
        };

        // Extract sign bit and encode mantissa
        // When mantissa is negative, use two's complement on 11 bits (add 2048)
        let (sign_bit, mantissa_bits) = if mantissa < 0 {
            (1u16, ((mantissa + 2048) as u16) & 0x07FF)
        } else {
            (0u16, (mantissa as u16) & 0x07FF)
        };

        // Build the 16-bit value: SEEE EMMM MMMM MMMM
        let value_u16 = (sign_bit << 15) | (u16::from(exponent) << 11) | mantissa_bits;

        let bytes = value_u16.to_be_bytes();
        buf[0] = bytes[0];
        buf[1] = bytes[1];
        Ok(2)
    }
}

impl DptDecode<f32> for Dpt9 {
    fn decode(&self, data: &[u8]) -> Result<f32> {
        self.decode_from_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float_eq(a: f32, b: f32, epsilon: f32) {
        assert!(
            (a - b).abs() < epsilon,
            "Expected {} ≈ {}, diff = {}",
            a,
            b,
            (a - b).abs()
        );
    }

    #[test]
    fn test_encode_zero() {
        let mut buf = [0u8; 2];
        let len = Dpt9::Temperature.encode(0.0, &mut buf).unwrap();
        assert_eq!(len, 2);
        assert_eq!(&buf[..len], &[0x00, 0x00]);
    }

    #[test]
    fn test_decode_zero() {
        let value = Dpt9::Temperature.decode_from_bytes(&[0x00, 0x00]).unwrap();
        assert_eq!(value, 0.0);
    }

    #[test]
    fn test_encode_positive_small() {
        // 21.5°C
        // 21.5 = 0.01 * M * 2^E
        // With E=2: M = 21.5 / 0.04 = 537.5 → 538 = 0x21A
        // Result: (2 << 11) | 0x21A = 0x121A
        let mut buf = [0u8; 2];
        let len = Dpt9::Temperature.encode(21.5, &mut buf).unwrap();
        assert_eq!(len, 2);
        let decoded = Dpt9::Temperature.decode(&buf[..len]).unwrap();
        // Just verify round-trip is close
        assert_float_eq(decoded, 21.5, 0.1);
    }

    #[test]
    fn test_encode_negative() {
        // -5.0°C
        let mut buf = [0u8; 2];
        let len = Dpt9::Temperature.encode(-5.0, &mut buf).unwrap();
        assert_eq!(len, 2);
        // Expected: mantissa = -500 = 0xFE0C (in 11-bit two's complement: 0x60C)
        // With sign bit: 0x860C
        let value = Dpt9::Temperature.decode(&buf[..len]).unwrap();
        assert_float_eq(value, -5.0, 0.01);
    }

    #[test]
    fn test_round_trip_temperature() {
        let mut buf = [0u8; 2];
        let test_values = [0.0, 10.5, 21.0, -10.0, 50.0, -273.0];

        for &value in &test_values {
            let len = Dpt9::Temperature.encode(value, &mut buf).unwrap();
            assert_eq!(len, 2);
            let decoded = Dpt9::Temperature.decode(&buf[..len]).unwrap();
            assert_float_eq(decoded, value, 0.1);
        }
    }

    #[test]
    fn test_round_trip_large_value() {
        // 1000.0 lux
        let mut buf = [0u8; 2];
        let len = Dpt9::Illuminance.encode(1000.0, &mut buf).unwrap();
        assert_eq!(len, 2);
        let decoded = Dpt9::Illuminance.decode(&buf[..len]).unwrap();
        assert_float_eq(decoded, 1000.0, 5.0);
    }

    #[test]
    fn test_round_trip_very_large_value() {
        // 100000.0 Pa (100 kPa)
        let mut buf = [0u8; 2];
        let len = Dpt9::Pressure.encode(100000.0, &mut buf).unwrap();
        assert_eq!(len, 2);
        let decoded = Dpt9::Pressure.decode(&buf[..len]).unwrap();
        assert_float_eq(decoded, 100000.0, 500.0);
    }

    #[test]
    fn test_encode_small_decimal() {
        // 0.5°C
        let mut buf = [0u8; 2];
        let len = Dpt9::Temperature.encode(0.5, &mut buf).unwrap();
        assert_eq!(len, 2);
        let decoded = Dpt9::Temperature.decode(&buf[..len]).unwrap();
        assert_float_eq(decoded, 0.5, 0.01);
    }

    #[test]
    fn test_round_trip_humidity() {
        // 65.5%
        let mut buf = [0u8; 2];
        let len = Dpt9::Humidity.encode(65.5, &mut buf).unwrap();
        assert_eq!(len, 2);
        let decoded = Dpt9::Humidity.decode(&buf[..len]).unwrap();
        assert_float_eq(decoded, 65.5, 0.5);
    }

    #[test]
    fn test_round_trip_wind_speed() {
        // 12.3 m/s
        let mut buf = [0u8; 2];
        let len = Dpt9::WindSpeed.encode(12.3, &mut buf).unwrap();
        assert_eq!(len, 2);
        let decoded = Dpt9::WindSpeed.decode(&buf[..len]).unwrap();
        assert_float_eq(decoded, 12.3, 0.2);
    }

    #[test]
    fn test_decode_invalid_length() {
        let result = Dpt9::Temperature.decode_from_bytes(&[0x00]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_decode_empty() {
        let result = Dpt9::Temperature.decode_from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_identifier() {
        assert_eq!(Dpt9::Temperature.identifier(), "9.001");
        assert_eq!(Dpt9::Illuminance.identifier(), "9.004");
        assert_eq!(Dpt9::Pressure.identifier(), "9.006");
    }

    #[test]
    fn test_unit() {
        assert_eq!(Dpt9::Temperature.unit(), "°C");
        assert_eq!(Dpt9::Illuminance.unit(), "lux");
        assert_eq!(Dpt9::Humidity.unit(), "%");
        assert_eq!(Dpt9::WindSpeed.unit(), "m/s");
    }

    #[test]
    fn test_round_trip_precision() {
        // Test various values for round-trip accuracy
        let mut buf = [0u8; 2];
        let test_values = [20.48, 10.76, -100.0, 0.5, -0.5];
        for &value in &test_values {
            let len = Dpt9::Temperature.encode(value, &mut buf).unwrap();
            assert_eq!(len, 2);
            let decoded = Dpt9::Temperature.decode(&buf[..len]).unwrap();
            // Allow some tolerance due to limited precision
            assert_float_eq(decoded, value, value.abs() * 0.01 + 0.1);
        }
    }

    #[test]
    fn test_decode_real_knx_bus_value() {
        // Real KNX bus value: 0x0C38 should decode to 21.6°C
        // This was incorrectly decoded as -19.5°C before the fix
        // Binary: 0000 1100 0011 1000
        // Sign: 0, Exponent: 1, Mantissa: 1080
        // Value = (1080 * 0.01) * 2^1 = 21.6
        let value = Dpt9::Temperature.decode_from_bytes(&[0x0C, 0x38]).unwrap();
        assert_float_eq(value, 21.6, 0.01);
    }

    #[test]
    fn test_decode_official_knx_example() {
        // Official KNX specification example: 0x0AF0 should decode to 15.04
        // Binary: 0000 1010 1111 0000
        // Sign: 0, Exponent: 1, Mantissa: 752
        // Value = (752 * 0.01) * 2^1 = 15.04
        let value = Dpt9::Temperature.decode_from_bytes(&[0x0A, 0xF0]).unwrap();
        assert_float_eq(value, 15.04, 0.01);
    }

    #[test]
    fn test_decode_negative_temperature() {
        // Test negative value decoding with sign bit
        // Encode -5.0°C and verify it decodes correctly
        let mut buf = [0u8; 2];
        let len = Dpt9::Temperature.encode(-5.0, &mut buf).unwrap();
        assert_eq!(len, 2);

        // Verify sign bit is set (bit 15 should be 1)
        assert_eq!(buf[0] & 0x80, 0x80, "Sign bit should be set for negative values");

        let decoded = Dpt9::Temperature.decode(&buf[..len]).unwrap();
        assert_float_eq(decoded, -5.0, 0.01);
    }

    // =========================================================================
    // DptEncode Trait Tests
    // =========================================================================

    #[test]
    fn test_trait_encode_basic() {
        let mut buf = [0u8; 2];

        let len = Dpt9::Temperature.encode(21.5, &mut buf).unwrap();
        assert_eq!(len, 2);

        let decoded = Dpt9::Temperature.decode(&buf).unwrap();
        assert_float_eq(decoded, 21.5, 0.1);
    }

    #[test]
    fn test_trait_encode_zero() {
        let mut buf = [0u8; 2];

        let len = Dpt9::Temperature.encode(0.0, &mut buf).unwrap();
        assert_eq!(len, 2);
        assert_eq!(buf, [0x00, 0x00]);
    }

    #[test]
    fn test_trait_encode_buffer_too_small() {
        let mut buf = [0u8; 1];
        let result = Dpt9::Temperature.encode(21.5, &mut buf);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Transport(_)));

        let mut buf = [0u8; 0];
        let result = Dpt9::Temperature.encode(21.5, &mut buf);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Transport(_)));
    }

    #[test]
    fn test_trait_encode_round_trip() {
        let mut buf = [0u8; 2];
        let test_values = [0.0, 10.5, 21.0, -10.0, 50.0, -273.0, 1000.0];

        for &value in &test_values {
            let len = Dpt9::Temperature.encode(value, &mut buf).unwrap();
            assert_eq!(len, 2);

            let decoded = Dpt9::Temperature.decode(&buf[..len]).unwrap();
            assert_float_eq(decoded, value, value.abs() * 0.01 + 0.5);
        }
    }

    #[test]
    fn test_trait_encode_negative() {
        let mut buf = [0u8; 2];

        let len = Dpt9::Temperature.encode(-5.0, &mut buf).unwrap();
        assert_eq!(len, 2);

        let decoded = Dpt9::Temperature.decode(&buf).unwrap();
        assert_float_eq(decoded, -5.0, 0.01);
    }

    #[test]
    fn test_trait_encode_large_values() {
        let mut buf = [0u8; 2];

        // 100000 Pa
        let len = Dpt9::Pressure.encode(100000.0, &mut buf).unwrap();
        assert_eq!(len, 2);

        let decoded = Dpt9::Pressure.decode(&buf).unwrap();
        assert_float_eq(decoded, 100000.0, 500.0);
    }
}
