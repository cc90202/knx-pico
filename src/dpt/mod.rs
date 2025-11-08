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
//! use knx_pico::dpt::{Dpt1, Dpt3, Dpt5, Dpt7, Dpt9, Dpt13, StepCode, DptEncode, DptDecode};
//!
//! // All DPT types now use the same pattern: encode to buffer, decode from slice
//! let mut buf = [0u8; 4];
//!
//! // Boolean value (1 byte)
//! let len = Dpt1::Switch.encode(true, &mut buf)?;
//! let value = Dpt1::Switch.decode(&buf[..len])?;
//!
//! // 3-bit controlled dimming/blind (1 byte)
//! let len = Dpt3::Dimming.encode((true, StepCode::Intervals4), &mut buf)?;
//! let cmd = Dpt3::Dimming.decode(&buf[..len])?;
//!
//! // Percentage 0-100% (1 byte)
//! let len = Dpt5::Percentage.encode(75, &mut buf)?;
//! let value = Dpt5::Percentage.decode(&buf[..len])?;
//!
//! // Brightness in lux (2 bytes)
//! let len = Dpt7::Brightness.encode(5000, &mut buf)?;
//! let lux = Dpt7::Brightness.decode(&buf[..len])?;
//!
//! // Temperature in °C (2 bytes)
//! let len = Dpt9::Temperature.encode(21.5, &mut buf)?;
//! let temp = Dpt9::Temperature.decode(&buf[..len])?;
//!
//! // Active energy in Wh (4 bytes)
//! let len = Dpt13::ActiveEnergy.encode(500000, &mut buf)?;
//! let wh = Dpt13::ActiveEnergy.decode(&buf[..len])?;
//! ```
//!
//! ## Design Note
//!
//! The `DptEncode` trait accepts an output buffer and returns the number of bytes written.
//! This design allows all DPT types to implement the trait consistently without requiring
//! static allocations for every possible value, solving the Liskov Substitution Principle
//! violation that existed in the previous `&'static [u8]` design.

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
/// This trait accepts an output buffer and returns the number of bytes written.
/// This design allows all DPT types to implement the trait consistently without
/// requiring static allocations for all possible values.
pub trait DptEncode<T> {
    /// Encode a value to KNX byte representation
    ///
    /// # Arguments
    /// * `value` - The value to encode
    /// * `buf` - Output buffer to write the encoded bytes
    ///
    /// # Returns
    /// The number of bytes written to the buffer
    ///
    /// # Errors
    /// Returns `BufferTooSmall` if the buffer is not large enough for the encoded data
    /// Returns `DptValueOutOfRange` if the value is outside the valid range
    fn encode(&self, value: T, buf: &mut [u8]) -> Result<usize>;
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
