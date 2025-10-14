//! DPT 3.xxx - 3-bit controlled (dimming and blinds)
//!
//! Control datapoint types for stepwise dimming and blind positioning.
//!
//! ## Format
//!
//! 4 bits total:
//! - Bit 3 (MSB): Control bit (direction)
//! - Bits 0-2 (LSB): Stepcode (0-7)
//!
//! ```text
//! ┌─────────┬─────────────┐
//! │ Control │  Stepcode   │
//! │  (1b)   │    (3b)     │
//! └─────────┴─────────────┘
//!    Bit 3     Bits 0-2
//! ```
//!
//! ## Stepcode Values
//!
//! - **0**: Break/Stop - halts current operation
//! - **1-7**: Intervals (1, 2, 4, 8, 16, 32, 64 intervals respectively)
//!
//! ## Common Subtypes
//!
//! - **3.007** - Dimming control (decrease/increase light intensity)
//! - **3.008** - Blind control (up/down blind positioning)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_rs::dpt::{Dpt3, StepCode};
//!
//! // Increase dimming by 4 intervals
//! let cmd = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals4)?;
//!
//! // Stop dimming
//! let stop = Dpt3::Dimming.encode_to_byte(false, StepCode::Break)?;
//!
//! // Move blind down by 1 interval
//! let down = Dpt3::Blind.encode_to_byte(true, StepCode::Intervals1)?;
//! ```

use crate::error::{KnxError, Result};

/// DPT 3.xxx 3-bit controlled types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt3 {
    /// DPT 3.007 - Dimming control (decrease/increase)
    Dimming,
    /// DPT 3.008 - Blind control (up/down)
    Blind,
}

/// Stepcode values for 3-bit controlled types
///
/// Represents the number of intervals for the control operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StepCode {
    /// Break/Stop - halts the current operation
    Break = 0,
    /// 1 interval
    Intervals1 = 1,
    /// 2 intervals
    Intervals2 = 2,
    /// 4 intervals
    Intervals4 = 3,
    /// 8 intervals
    Intervals8 = 4,
    /// 16 intervals
    Intervals16 = 5,
    /// 32 intervals
    Intervals32 = 6,
    /// 64 intervals (maximum)
    Intervals64 = 7,
}

/// Control direction for DPT 3 operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlCommand {
    /// Control bit: false = decrease/up, true = increase/down
    pub control: bool,
    /// Stepcode (0-7)
    pub stepcode: StepCode,
}

impl Dpt3 {
    /// Encode a 3-bit controlled command to a byte
    ///
    /// # Arguments
    ///
    /// * `control` - Control direction (false = decrease/up, true = increase/down)
    /// * `stepcode` - Step intervals (0-7)
    ///
    /// # Returns
    ///
    /// A single byte with the encoded command
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use knx_rs::dpt::{Dpt3, StepCode};
    ///
    /// // Increase dimming by 4 intervals
    /// let byte = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals4)?;
    /// assert_eq!(byte, 0x0B); // 0b00001011
    /// # Ok::<(), knx_rs::KnxError>(())
    /// ```
    #[inline]
    pub fn encode_to_byte(&self, control: bool, stepcode: StepCode) -> Result<u8> {
        let control_bit = if control { 0x08 } else { 0x00 };
        let step_bits = stepcode as u8 & 0x07;
        Ok(control_bit | step_bits)
    }

    /// Decode a byte to a control command
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing the command (at least 1 byte)
    ///
    /// # Returns
    ///
    /// A `ControlCommand` with the decoded control and stepcode
    ///
    /// # Errors
    ///
    /// Returns `KnxError::InvalidDptData` if data is empty or stepcode is invalid
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use knx_rs::dpt::{Dpt3, StepCode};
    ///
    /// let cmd = Dpt3::Dimming.decode(&[0x0B])?;
    /// assert_eq!(cmd.control, true);
    /// assert_eq!(cmd.stepcode, StepCode::Intervals4);
    /// # Ok::<(), knx_rs::KnxError>(())
    /// ```
    pub fn decode(&self, data: &[u8]) -> Result<ControlCommand> {
        if data.is_empty() {
            return Err(KnxError::invalid_dpt_data());
        }

        let byte = data[0];
        let control = (byte & 0x08) != 0;
        let stepcode_value = byte & 0x07;

        let stepcode = StepCode::from_u8(stepcode_value)?;

        Ok(ControlCommand { control, stepcode })
    }

    /// Get the DPT identifier string (e.g., "3.007")
    pub const fn identifier(&self) -> &'static str {
        match self {
            Dpt3::Dimming => "3.007",
            Dpt3::Blind => "3.008",
        }
    }

    /// Get semantic labels for control directions
    ///
    /// Returns a tuple (`control_false_label`, `control_true_label`)
    pub const fn control_labels(&self) -> (&'static str, &'static str) {
        match self {
            Dpt3::Dimming => ("decrease", "increase"),
            Dpt3::Blind => ("up", "down"),
        }
    }
}

impl StepCode {
    /// Convert a u8 value to a StepCode
    ///
    /// # Errors
    ///
    /// Returns `KnxError::InvalidDptData` if value is not in range 0-7
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(StepCode::Break),
            1 => Ok(StepCode::Intervals1),
            2 => Ok(StepCode::Intervals2),
            3 => Ok(StepCode::Intervals4),
            4 => Ok(StepCode::Intervals8),
            5 => Ok(StepCode::Intervals16),
            6 => Ok(StepCode::Intervals32),
            7 => Ok(StepCode::Intervals64),
            _ => Err(KnxError::invalid_dpt_data()),
        }
    }

    /// Get the number of intervals this stepcode represents
    ///
    /// Returns 0 for Break, otherwise 1, 2, 4, 8, 16, 32, or 64
    pub const fn intervals(&self) -> u8 {
        match self {
            StepCode::Break => 0,
            StepCode::Intervals1 => 1,
            StepCode::Intervals2 => 2,
            StepCode::Intervals4 => 4,
            StepCode::Intervals8 => 8,
            StepCode::Intervals16 => 16,
            StepCode::Intervals32 => 32,
            StepCode::Intervals64 => 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_break() {
        // Break with control=false (0b0000_0000 = 0x00)
        let byte = Dpt3::Dimming.encode_to_byte(false, StepCode::Break).unwrap();
        assert_eq!(byte, 0x00);

        // Break with control=true (0b0000_1000 = 0x08)
        let byte = Dpt3::Dimming.encode_to_byte(true, StepCode::Break).unwrap();
        assert_eq!(byte, 0x08);
    }

    #[test]
    fn test_encode_dimming_decrease() {
        // Decrease by 1 interval: control=false, stepcode=1
        // 0b0000_0001 = 0x01
        let byte = Dpt3::Dimming.encode_to_byte(false, StepCode::Intervals1).unwrap();
        assert_eq!(byte, 0x01);

        // Decrease by 4 intervals: control=false, stepcode=3
        // 0b0000_0011 = 0x03
        let byte = Dpt3::Dimming.encode_to_byte(false, StepCode::Intervals4).unwrap();
        assert_eq!(byte, 0x03);
    }

    #[test]
    fn test_encode_dimming_increase() {
        // Increase by 1 interval: control=true, stepcode=1
        // 0b0000_1001 = 0x09
        let byte = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals1).unwrap();
        assert_eq!(byte, 0x09);

        // Increase by 4 intervals: control=true, stepcode=3
        // 0b0000_1011 = 0x0B
        let byte = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals4).unwrap();
        assert_eq!(byte, 0x0B);
    }

    #[test]
    fn test_encode_blind_up() {
        // Move up by 2 intervals: control=false, stepcode=2
        // 0b0000_0010 = 0x02
        let byte = Dpt3::Blind.encode_to_byte(false, StepCode::Intervals2).unwrap();
        assert_eq!(byte, 0x02);
    }

    #[test]
    fn test_encode_blind_down() {
        // Move down by 8 intervals: control=true, stepcode=4
        // 0b0000_1100 = 0x0C
        let byte = Dpt3::Blind.encode_to_byte(true, StepCode::Intervals8).unwrap();
        assert_eq!(byte, 0x0C);
    }

    #[test]
    fn test_encode_all_stepcodes() {
        let stepcodes = [
            (StepCode::Break, 0),
            (StepCode::Intervals1, 1),
            (StepCode::Intervals2, 2),
            (StepCode::Intervals4, 3),
            (StepCode::Intervals8, 4),
            (StepCode::Intervals16, 5),
            (StepCode::Intervals32, 6),
            (StepCode::Intervals64, 7),
        ];

        for (stepcode, expected_bits) in &stepcodes {
            // Control = false
            let byte = Dpt3::Dimming.encode_to_byte(false, *stepcode).unwrap();
            assert_eq!(byte, *expected_bits);

            // Control = true
            let byte = Dpt3::Dimming.encode_to_byte(true, *stepcode).unwrap();
            assert_eq!(byte, 0x08 | expected_bits);
        }
    }

    // =========================================================================
    // Decoding Tests
    // =========================================================================

    #[test]
    fn test_decode_break() {
        let cmd = Dpt3::Dimming.decode(&[0x00]).unwrap();
        assert_eq!(cmd.control, false);
        assert_eq!(cmd.stepcode, StepCode::Break);

        let cmd = Dpt3::Dimming.decode(&[0x08]).unwrap();
        assert_eq!(cmd.control, true);
        assert_eq!(cmd.stepcode, StepCode::Break);
    }

    #[test]
    fn test_decode_dimming_decrease() {
        // Decrease by 1 interval
        let cmd = Dpt3::Dimming.decode(&[0x01]).unwrap();
        assert_eq!(cmd.control, false);
        assert_eq!(cmd.stepcode, StepCode::Intervals1);

        // Decrease by 4 intervals
        let cmd = Dpt3::Dimming.decode(&[0x03]).unwrap();
        assert_eq!(cmd.control, false);
        assert_eq!(cmd.stepcode, StepCode::Intervals4);
    }

    #[test]
    fn test_decode_dimming_increase() {
        // Increase by 1 interval
        let cmd = Dpt3::Dimming.decode(&[0x09]).unwrap();
        assert_eq!(cmd.control, true);
        assert_eq!(cmd.stepcode, StepCode::Intervals1);

        // Increase by 4 intervals
        let cmd = Dpt3::Dimming.decode(&[0x0B]).unwrap();
        assert_eq!(cmd.control, true);
        assert_eq!(cmd.stepcode, StepCode::Intervals4);
    }

    #[test]
    fn test_decode_blind_up() {
        let cmd = Dpt3::Blind.decode(&[0x02]).unwrap();
        assert_eq!(cmd.control, false);
        assert_eq!(cmd.stepcode, StepCode::Intervals2);
    }

    #[test]
    fn test_decode_blind_down() {
        let cmd = Dpt3::Blind.decode(&[0x0C]).unwrap();
        assert_eq!(cmd.control, true);
        assert_eq!(cmd.stepcode, StepCode::Intervals8);
    }

    #[test]
    fn test_decode_all_combinations() {
        // Test all 16 possible combinations (2 control × 8 stepcodes)
        for byte in 0x00..=0x0F {
            let cmd = Dpt3::Dimming.decode(&[byte]).unwrap();

            let expected_control = (byte & 0x08) != 0;
            let expected_stepcode = StepCode::from_u8(byte & 0x07).unwrap();

            assert_eq!(cmd.control, expected_control);
            assert_eq!(cmd.stepcode, expected_stepcode);
        }
    }

    #[test]
    fn test_decode_empty_data() {
        let result = Dpt3::Dimming.decode(&[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_decode_ignores_upper_bits() {
        // Upper 4 bits should be ignored
        let cmd = Dpt3::Dimming.decode(&[0xFF]).unwrap();
        assert_eq!(cmd.control, true);  // Bit 3 is set
        assert_eq!(cmd.stepcode, StepCode::Intervals64);  // Bits 0-2 are 111

        let cmd = Dpt3::Dimming.decode(&[0xF0]).unwrap();
        assert_eq!(cmd.control, false);  // Bit 3 is not set
        assert_eq!(cmd.stepcode, StepCode::Break);  // Bits 0-2 are 000
    }

    // =========================================================================
    // Round-trip Tests
    // =========================================================================

    #[test]
    fn test_round_trip() {
        let test_cases = [
            (false, StepCode::Break),
            (true, StepCode::Break),
            (false, StepCode::Intervals1),
            (true, StepCode::Intervals1),
            (false, StepCode::Intervals4),
            (true, StepCode::Intervals4),
            (false, StepCode::Intervals64),
            (true, StepCode::Intervals64),
        ];

        for (control, stepcode) in &test_cases {
            // Encode
            let byte = Dpt3::Dimming.encode_to_byte(*control, *stepcode).unwrap();

            // Decode
            let cmd = Dpt3::Dimming.decode(&[byte]).unwrap();

            // Verify
            assert_eq!(cmd.control, *control);
            assert_eq!(cmd.stepcode, *stepcode);
        }
    }

    // =========================================================================
    // StepCode Tests
    // =========================================================================

    #[test]
    fn test_stepcode_from_u8_valid() {
        assert_eq!(StepCode::from_u8(0).unwrap(), StepCode::Break);
        assert_eq!(StepCode::from_u8(1).unwrap(), StepCode::Intervals1);
        assert_eq!(StepCode::from_u8(2).unwrap(), StepCode::Intervals2);
        assert_eq!(StepCode::from_u8(3).unwrap(), StepCode::Intervals4);
        assert_eq!(StepCode::from_u8(4).unwrap(), StepCode::Intervals8);
        assert_eq!(StepCode::from_u8(5).unwrap(), StepCode::Intervals16);
        assert_eq!(StepCode::from_u8(6).unwrap(), StepCode::Intervals32);
        assert_eq!(StepCode::from_u8(7).unwrap(), StepCode::Intervals64);
    }

    #[test]
    fn test_stepcode_from_u8_invalid() {
        let result = StepCode::from_u8(8);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::Dpt(_)));
    }

    #[test]
    fn test_stepcode_intervals() {
        assert_eq!(StepCode::Break.intervals(), 0);
        assert_eq!(StepCode::Intervals1.intervals(), 1);
        assert_eq!(StepCode::Intervals2.intervals(), 2);
        assert_eq!(StepCode::Intervals4.intervals(), 4);
        assert_eq!(StepCode::Intervals8.intervals(), 8);
        assert_eq!(StepCode::Intervals16.intervals(), 16);
        assert_eq!(StepCode::Intervals32.intervals(), 32);
        assert_eq!(StepCode::Intervals64.intervals(), 64);
    }

    // =========================================================================
    // Metadata Tests
    // =========================================================================

    #[test]
    fn test_identifier() {
        assert_eq!(Dpt3::Dimming.identifier(), "3.007");
        assert_eq!(Dpt3::Blind.identifier(), "3.008");
    }

    #[test]
    fn test_control_labels() {
        assert_eq!(Dpt3::Dimming.control_labels(), ("decrease", "increase"));
        assert_eq!(Dpt3::Blind.control_labels(), ("up", "down"));
    }

    // =========================================================================
    // Semantic Tests
    // =========================================================================

    #[test]
    fn test_semantic_dimming() {
        // Start dimming up
        let start_up = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals1).unwrap();
        let cmd = Dpt3::Dimming.decode(&[start_up]).unwrap();
        assert_eq!(cmd.control, true);  // increase
        assert_eq!(cmd.stepcode.intervals(), 1);

        // Stop dimming
        let stop = Dpt3::Dimming.encode_to_byte(false, StepCode::Break).unwrap();
        let cmd = Dpt3::Dimming.decode(&[stop]).unwrap();
        assert_eq!(cmd.stepcode, StepCode::Break);
    }

    #[test]
    fn test_semantic_blind() {
        // Move blind down
        let down = Dpt3::Blind.encode_to_byte(true, StepCode::Intervals8).unwrap();
        let cmd = Dpt3::Blind.decode(&[down]).unwrap();
        assert_eq!(cmd.control, true);  // down
        assert_eq!(cmd.stepcode.intervals(), 8);

        // Stop blind
        let stop = Dpt3::Blind.encode_to_byte(false, StepCode::Break).unwrap();
        let cmd = Dpt3::Blind.decode(&[stop]).unwrap();
        assert_eq!(cmd.stepcode, StepCode::Break);
    }
}
