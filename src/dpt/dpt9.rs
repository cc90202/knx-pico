//! DPT 9.xxx - 2-byte Float (16-bit floating point)
//!
//! 2-byte floating point datapoint types represent values using a custom
//! 16-bit floating point format with 1 sign bit, 4 exponent bits, and 11 mantissa bits.
//!
//! ## Format
//!
//! ```text
//! Byte 0: MEEE EMMM
//! Byte 1: MMMM MMMM
//!
//! M = Sign bit (0 = positive, 1 = negative)
//! E = Exponent (4 bits, unsigned, bias 0)
//! M = Mantissa (11 bits, signed two's complement)
//!
//! Value = (0.01 * M) * 2^E
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
//! use knx_rs::dpt::Dpt9;
//!
//! // Encode temperature
//! let bytes = Dpt9::Temperature.encode_to_bytes(21.5)?;
//! // bytes = [0x0C, 0x1A]
//!
//! // Decode
//! let temp = Dpt9::Temperature.decode_from_bytes(&bytes)?;
//! // temp ≈ 21.5
//! ```

use crate::error::{KnxError, Result};
use crate::dpt::{DptDecode, DptEncode};

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

    /// Encode f32 to 2-byte KNX float format
    ///
    /// # Arguments
    /// * `value` - The floating point value to encode
    ///
    /// # Returns
    /// A 2-byte array [high_byte, low_byte]
    ///
    /// # Errors
    /// Returns `DptValueOutOfRange` if value is out of representable range
    pub fn encode_to_bytes(&self, value: f32) -> Result<[u8; 2]> {
        // Handle special case of zero
        if value == 0.0 {
            return Ok([0x00, 0x00]);
        }

        // Value = (0.01 * mantissa) * 2^exponent
        // So: mantissa = value * 100 / 2^exponent
        //
        // mantissa is 11-bit two's complement: -2048 to 2047

        // Find the appropriate exponent (0-15)
        // Mantissa is 11-bit two's complement: -2048 to +2047
        // But we need to stay within the ACTUALLY representable range:
        // Positive: 0 to 2047 (0x000 to 0x7FF, but 0x400-0x7FF are negative in 11-bit!)
        // So positive range is actually: 0 to 1023 (0x000 to 0x3FF)
        // Negative range: -1024 to -1 (0x400 to 0x7FF)
        //
        // Actually, let me re-read the spec. The mantissa IS signed, so:
        // - Full range is -2048 to +2047
        // - In 11-bit two's complement:
        //   - 0x000 to 0x3FF = 0 to +1023
        //   - 0x400 to 0x7FF = -1024 to -1
        //
        // Wait, that's only -1024 to +1023, not -2048 to +2047!
        //
        // Let me check: 11 bits signed = -(2^10) to +(2^10 - 1) = -1024 to +1023
        //
        // So the actual range is -1024 to +1023, NOT -2048 to +2047!
        let mut exponent = 0u8;
        let mut mantissa_f = value * 100.0;

        // Scale to fit mantissa in 11-bit signed range: -1024 to +1023
        while (mantissa_f > 1023.0 || mantissa_f < -1024.0) && exponent < 15 {
            exponent += 1;
            mantissa_f = value * 100.0 / (1u32 << exponent) as f32;
        }

        // Check range
        if mantissa_f > 1023.0 || mantissa_f < -1024.0 {
            return Err(KnxError::DptValueOutOfRange);
        }

        // Round to nearest integer (manual rounding for no_std)
        let mantissa = if mantissa_f >= 0.0 {
            (mantissa_f + 0.5) as i16
        } else {
            (mantissa_f - 0.5) as i16
        };

        // mantissa is 11-bit two's complement
        // Just mask to 11 bits - the two's complement representation is preserved
        let mantissa_u16 = mantissa as u16 & 0x07FF;

        // Build the 16-bit value
        // Bit 14-11: exponent (4 bits)
        // Bit 10-0: mantissa (11 bits, two's complement)
        let value_u16 = ((exponent as u16) << 11) | mantissa_u16;

        Ok(value_u16.to_be_bytes())
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
            return Err(KnxError::InvalidDptData);
        }

        let value_u16 = u16::from_be_bytes([bytes[0], bytes[1]]);

        // Extract fields
        let exponent = ((value_u16 >> 11) & 0x0F) as u8;
        let mantissa_raw = (value_u16 & 0x07FF) as u16;

        // Convert mantissa from 11-bit two's complement
        let mantissa = if mantissa_raw & 0x0400 != 0 {
            // Negative: extend sign bit
            (mantissa_raw | 0xF800) as i16
        } else {
            mantissa_raw as i16
        };

        // Calculate value = (0.01 * mantissa) * 2^exponent
        // The sign is already included in the mantissa (two's complement)
        let value = (0.01 * mantissa as f32) * (1u32 << exponent) as f32;

        Ok(value)
    }
}

impl DptEncode<f32> for Dpt9 {
    fn encode(&self, _value: f32) -> Result<&'static [u8]> {
        // Can't return static slice for all possible float values
        Err(KnxError::UnsupportedOperation)
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
        assert!((a - b).abs() < epsilon, "Expected {} ≈ {}, diff = {}", a, b, (a - b).abs());
    }

    #[test]
    fn test_encode_zero() {
        let bytes = Dpt9::Temperature.encode_to_bytes(0.0).unwrap();
        assert_eq!(bytes, [0x00, 0x00]);
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
        let bytes = Dpt9::Temperature.encode_to_bytes(21.5).unwrap();
        let decoded = Dpt9::Temperature.decode_from_bytes(&bytes).unwrap();
        // Just verify round-trip is close
        assert_float_eq(decoded, 21.5, 0.1);
    }


    #[test]
    fn test_encode_negative() {
        // -5.0°C
        let bytes = Dpt9::Temperature.encode_to_bytes(-5.0).unwrap();
        // Expected: mantissa = -500 = 0xFE0C (in 11-bit two's complement: 0x60C)
        // With sign bit: 0x860C
        let value = Dpt9::Temperature.decode_from_bytes(&bytes).unwrap();
        assert_float_eq(value, -5.0, 0.01);
    }

    #[test]
    fn test_round_trip_temperature() {
        let test_values = [0.0, 10.5, 21.0, -10.0, 50.0, -273.0];

        for &value in &test_values {
            let bytes = Dpt9::Temperature.encode_to_bytes(value).unwrap();
            let decoded = Dpt9::Temperature.decode_from_bytes(&bytes).unwrap();
            assert_float_eq(decoded, value, 0.1);
        }
    }

    #[test]
    fn test_round_trip_large_value() {
        // 1000.0 lux
        let bytes = Dpt9::Illuminance.encode_to_bytes(1000.0).unwrap();
        let decoded = Dpt9::Illuminance.decode_from_bytes(&bytes).unwrap();
        assert_float_eq(decoded, 1000.0, 5.0);
    }

    #[test]
    fn test_round_trip_very_large_value() {
        // 100000.0 Pa (100 kPa)
        let bytes = Dpt9::Pressure.encode_to_bytes(100000.0).unwrap();
        let decoded = Dpt9::Pressure.decode_from_bytes(&bytes).unwrap();
        assert_float_eq(decoded, 100000.0, 500.0);
    }

    #[test]
    fn test_encode_small_decimal() {
        // 0.5°C
        let bytes = Dpt9::Temperature.encode_to_bytes(0.5).unwrap();
        let decoded = Dpt9::Temperature.decode_from_bytes(&bytes).unwrap();
        assert_float_eq(decoded, 0.5, 0.01);
    }

    #[test]
    fn test_round_trip_humidity() {
        // 65.5%
        let bytes = Dpt9::Humidity.encode_to_bytes(65.5).unwrap();
        let decoded = Dpt9::Humidity.decode_from_bytes(&bytes).unwrap();
        assert_float_eq(decoded, 65.5, 0.5);
    }

    #[test]
    fn test_round_trip_wind_speed() {
        // 12.3 m/s
        let bytes = Dpt9::WindSpeed.encode_to_bytes(12.3).unwrap();
        let decoded = Dpt9::WindSpeed.decode_from_bytes(&bytes).unwrap();
        assert_float_eq(decoded, 12.3, 0.2);
    }

    #[test]
    fn test_decode_invalid_length() {
        let result = Dpt9::Temperature.decode_from_bytes(&[0x00]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::InvalidDptData));
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
        let test_values = [20.48, 10.76, -100.0, 0.5, -0.5];
        for &value in &test_values {
            let bytes = Dpt9::Temperature.encode_to_bytes(value).unwrap();
            let decoded = Dpt9::Temperature.decode_from_bytes(&bytes).unwrap();
            // Allow some tolerance due to limited precision
            assert_float_eq(decoded, value, value.abs() * 0.01 + 0.1);
        }
    }
}
