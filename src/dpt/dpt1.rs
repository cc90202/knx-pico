//! DPT 1.xxx - Boolean (1-bit)
//!
//! Boolean datapoint types represent binary states (on/off, true/false, etc.)
//! encoded as a single bit (LSB of the data byte in APDU).
//!
//! ## Format
//!
//! - 6 bits: unused (always 0)
//! - 2 bits: data (only LSB used)
//!   - `0` = false/off/disable/...
//!   - `1` = true/on/enable/...
//!
//! ## Common Subtypes
//!
//! - **1.001** - Switch (off/on)
//! - **1.002** - Bool (false/true)
//! - **1.003** - Enable (disable/enable)
//! - **1.008** - UpDown (up/down)
//! - **1.009** - OpenClose (open/close)
//! - **1.010** - Start (stop/start)
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_rs::dpt::{Dpt1, DptEncode, DptDecode};
//!
//! // Turn on a switch
//! let data = Dpt1::Switch.encode(true)?;  // [0x01]
//!
//! // Decode
//! let state = Dpt1::Switch.decode(&[0x01])?;  // true
//! ```

use crate::error::{KnxError, Result};
use crate::dpt::{DptEncode, DptDecode};

/// DPT 1.xxx Boolean types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpt1 {
    /// DPT 1.001 - Switch (off/on)
    Switch,
    /// DPT 1.002 - Bool (false/true)
    Bool,
    /// DPT 1.003 - Enable (disable/enable)
    Enable,
    /// DPT 1.004 - Ramp (no ramp/ramp)
    Ramp,
    /// DPT 1.005 - Alarm (no alarm/alarm)
    Alarm,
    /// DPT 1.006 - BinaryValue (low/high)
    BinaryValue,
    /// DPT 1.007 - Step (decrease/increase)
    Step,
    /// DPT 1.008 - UpDown (up/down)
    UpDown,
    /// DPT 1.009 - OpenClose (open/close)
    OpenClose,
    /// DPT 1.010 - Start (stop/start)
    Start,
    /// DPT 1.011 - State (inactive/active)
    State,
    /// DPT 1.012 - Invert (not inverted/inverted)
    Invert,
}

// Static constants for encoded values
static DPT1_FALSE: [u8; 1] = [0x00];
static DPT1_TRUE: [u8; 1] = [0x01];

impl DptEncode<bool> for Dpt1 {
    fn encode(&self, value: bool) -> Result<&'static [u8]> {
        if value {
            Ok(&DPT1_TRUE)
        } else {
            Ok(&DPT1_FALSE)
        }
    }
}

impl DptDecode<bool> for Dpt1 {
    fn decode(&self, data: &[u8]) -> Result<bool> {
        if data.is_empty() {
            return Err(KnxError::InvalidDptData);
        }

        // Only the LSB matters, mask out upper bits
        let bit = data[0] & 0x01;
        Ok(bit != 0)
    }
}

impl Dpt1 {
    /// Get the DPT identifier string (e.g., "1.001")
    pub const fn identifier(&self) -> &'static str {
        match self {
            Dpt1::Switch => "1.001",
            Dpt1::Bool => "1.002",
            Dpt1::Enable => "1.003",
            Dpt1::Ramp => "1.004",
            Dpt1::Alarm => "1.005",
            Dpt1::BinaryValue => "1.006",
            Dpt1::Step => "1.007",
            Dpt1::UpDown => "1.008",
            Dpt1::OpenClose => "1.009",
            Dpt1::Start => "1.010",
            Dpt1::State => "1.011",
            Dpt1::Invert => "1.012",
        }
    }

    /// Get semantic labels for false/true values
    ///
    /// Returns a tuple (false_label, true_label)
    pub const fn labels(&self) -> (&'static str, &'static str) {
        match self {
            Dpt1::Switch => ("off", "on"),
            Dpt1::Bool => ("false", "true"),
            Dpt1::Enable => ("disable", "enable"),
            Dpt1::Ramp => ("no ramp", "ramp"),
            Dpt1::Alarm => ("no alarm", "alarm"),
            Dpt1::BinaryValue => ("low", "high"),
            Dpt1::Step => ("decrease", "increase"),
            Dpt1::UpDown => ("up", "down"),
            Dpt1::OpenClose => ("open", "close"),
            Dpt1::Start => ("stop", "start"),
            Dpt1::State => ("inactive", "active"),
            Dpt1::Invert => ("not inverted", "inverted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_false() {
        let result = Dpt1::Switch.encode(false).unwrap();
        assert_eq!(result, &[0x00]);
    }

    #[test]
    fn test_encode_true() {
        let result = Dpt1::Switch.encode(true).unwrap();
        assert_eq!(result, &[0x01]);
    }

    #[test]
    fn test_decode_false() {
        let result = Dpt1::Switch.decode(&[0x00]).unwrap();
        assert_eq!(result, false);
    }

    #[test]
    fn test_decode_true() {
        let result = Dpt1::Switch.decode(&[0x01]).unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn test_decode_with_upper_bits_set() {
        // Upper bits should be ignored
        let result = Dpt1::Switch.decode(&[0xFF]).unwrap();
        assert_eq!(result, true);

        let result = Dpt1::Switch.decode(&[0xFE]).unwrap();
        assert_eq!(result, false);
    }

    #[test]
    fn test_decode_empty_data() {
        let result = Dpt1::Switch.decode(&[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KnxError::InvalidDptData));
    }

    #[test]
    fn test_round_trip() {
        // false
        let encoded = Dpt1::Bool.encode(false).unwrap();
        let decoded = Dpt1::Bool.decode(encoded).unwrap();
        assert_eq!(decoded, false);

        // true
        let encoded = Dpt1::Bool.encode(true).unwrap();
        let decoded = Dpt1::Bool.decode(encoded).unwrap();
        assert_eq!(decoded, true);
    }

    #[test]
    fn test_all_subtypes_encode() {
        let subtypes = [
            Dpt1::Switch,
            Dpt1::Bool,
            Dpt1::Enable,
            Dpt1::Ramp,
            Dpt1::Alarm,
            Dpt1::BinaryValue,
            Dpt1::Step,
            Dpt1::UpDown,
            Dpt1::OpenClose,
            Dpt1::Start,
            Dpt1::State,
            Dpt1::Invert,
        ];

        for subtype in &subtypes {
            // All subtypes should encode the same way
            assert_eq!(subtype.encode(false).unwrap(), &[0x00]);
            assert_eq!(subtype.encode(true).unwrap(), &[0x01]);
        }
    }

    #[test]
    fn test_identifier() {
        assert_eq!(Dpt1::Switch.identifier(), "1.001");
        assert_eq!(Dpt1::Bool.identifier(), "1.002");
        assert_eq!(Dpt1::OpenClose.identifier(), "1.009");
    }

    #[test]
    fn test_labels() {
        assert_eq!(Dpt1::Switch.labels(), ("off", "on"));
        assert_eq!(Dpt1::Bool.labels(), ("false", "true"));
        assert_eq!(Dpt1::Enable.labels(), ("disable", "enable"));
        assert_eq!(Dpt1::UpDown.labels(), ("up", "down"));
        assert_eq!(Dpt1::OpenClose.labels(), ("open", "close"));
    }

    #[test]
    fn test_semantic_interpretation() {
        // Test that different DPT types have different meanings
        // but same encoding
        let switch_on = Dpt1::Switch.encode(true).unwrap();
        let door_close = Dpt1::OpenClose.encode(true).unwrap();

        // Same binary representation
        assert_eq!(switch_on, door_close);

        // But different semantic meaning
        assert_eq!(Dpt1::Switch.labels().1, "on");
        assert_eq!(Dpt1::OpenClose.labels().1, "close");
    }
}
