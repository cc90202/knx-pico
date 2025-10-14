//! KNX Datapoint Types (DPT)
//!
//! This module provides encoding and decoding for KNX Datapoint Types.
//! DPTs define how to interpret the data payload in KNX telegrams.
//!
//! ## Supported DPT Families
//!
//! - **DPT 1.xxx** - Boolean (1 bit): switches, buttons, binary sensors
//! - **DPT 3.xxx** - 3-bit controlled: dimming, blind control
//! - **DPT 5.xxx** - 8-bit unsigned: percentages, angles, counters
//! - **DPT 7.xxx** - 16-bit unsigned: pulses, brightness, color temperature
//! - **DPT 9.xxx** - 2-byte float: temperature, illuminance, pressure
//! - **DPT 13.xxx** - 32-bit signed: energy, flow rate, long counters
//!
//! ## Usage
//!
//! ```rust,no_run
//! use knx_rs::dpt::{Dpt1, Dpt3, Dpt5, Dpt7, Dpt9, Dpt13, StepCode, DptEncode, DptDecode};
//!
//! // Boolean value - uses encode() returning &'static [u8]
//! let data = Dpt1::Switch.encode(true)?;
//! let value = Dpt1::Switch.decode(&data)?;
//!
//! // For multi-byte types, use specific methods:
//!
//! // 3-bit controlled (dimming/blind) - returns owned byte
//! let byte = Dpt3::Dimming.encode_to_byte(true, StepCode::Intervals4)?;
//! let cmd = Dpt3::Dimming.decode(&[byte])?;
//!
//! // Percentage (0-100%) - returns owned byte
//! let byte = Dpt5::Percentage.encode_to_byte(75)?;
//! let value = Dpt5::Percentage.decode(&[byte])?;
//!
//! // Brightness (lux) - returns owned array
//! let bytes = Dpt7::Brightness.encode_to_bytes(5000)?;
//! let lux = Dpt7::Brightness.decode(&bytes)?;
//!
//! // Temperature (°C) - returns owned array
//! let bytes = Dpt9::Temperature.encode_to_bytes(21.5)?;
//! let temp = Dpt9::Temperature.decode_from_bytes(&bytes)?;
//!
//! // Active energy (Wh) - returns owned array
//! let bytes = Dpt13::ActiveEnergy.encode_to_bytes(500000)?;
//! let wh = Dpt13::ActiveEnergy.decode(&bytes)?;
//! ```
//!
//! ## Design Note
//!
//! The `DptEncode` trait returns `&'static [u8]` which works well for simple
//! types (like DPT1 with only 2 possible values), but not for types with many
//! possible values. For those, use the type-specific `encode_to_byte()` or
//! `encode_to_bytes()` methods that return owned data.

use crate::error::Result;

pub mod dpt1;
pub mod dpt3;
pub mod dpt5;
pub mod dpt7;
pub mod dpt9;
pub mod dpt13;

// Re-export common types
#[doc(inline)]
pub use dpt1::Dpt1;
#[doc(inline)]
pub use dpt3::{Dpt3, StepCode, ControlCommand};
#[doc(inline)]
pub use dpt5::Dpt5;
#[doc(inline)]
pub use dpt7::Dpt7;
#[doc(inline)]
pub use dpt9::Dpt9;
#[doc(inline)]
pub use dpt13::Dpt13;

/// Trait for encoding values to KNX data format
///
/// This trait is mainly for documentation and type checking.
/// Actual encoding should use the type-specific methods like
/// `encode_to_byte()`, `encode_to_bytes()`, etc.
pub trait DptEncode<T> {
    /// Encode a value to KNX byte representation
    ///
    /// # Arguments
    /// * `value` - The value to encode
    ///
    /// # Returns
    /// A byte array containing the encoded value
    ///
    /// # Note
    /// This method may not be implemented for all DPT types due to
    /// the limitation of returning `&'static [u8]`. Use type-specific
    /// methods like `encode_to_byte()` or `encode_to_bytes()` instead.
    fn encode(&self, value: T) -> Result<&'static [u8]>;
}

/// Trait for decoding KNX data to values
pub trait DptDecode<T> {
    /// Decode KNX byte representation to a value
    ///
    /// # Arguments
    /// * `data` - The byte slice to decode
    ///
    /// # Returns
    /// The decoded value
    fn decode(&self, data: &[u8]) -> Result<T>;
}
