//! KNX Datapoint Types (DPT)
//!
//! This module provides encoding and decoding for KNX Datapoint Types.
//! DPTs define how to interpret the data payload in KNX telegrams.
//!
//! ## Supported DPT Families
//!
//! - **DPT 1.xxx** - Boolean (1 bit): switches, buttons, binary sensors
//! - **DPT 5.xxx** - 8-bit unsigned: percentages, angles, counters
//! - **DPT 9.xxx** - 2-byte float: temperature, illuminance, pressure
//!
//! ## Usage
//!
//! ```rust,no_run
//! use knx_rs::dpt::{Dpt1, Dpt5, Dpt9, DptEncode, DptDecode};
//!
//! // Boolean value
//! let data = Dpt1::Switch.encode(true)?;
//! let value = Dpt1::Switch.decode(&data)?;
//!
//! // Percentage (0-100%)
//! let byte = Dpt5::Percentage.encode_to_byte(75)?;
//! let value = Dpt5::Percentage.decode(&[byte])?;
//!
//! // Temperature (°C)
//! let bytes = Dpt9::Temperature.encode_to_bytes(21.5)?;
//! let temp = Dpt9::Temperature.decode_from_bytes(&bytes)?;
//! ```

use crate::error::Result;

pub mod dpt1;
pub mod dpt5;
pub mod dpt9;

// Re-export common types
pub use dpt1::Dpt1;
pub use dpt5::Dpt5;
pub use dpt9::Dpt9;

/// Trait for encoding values to KNX data format
pub trait DptEncode<T> {
    /// Encode a value to KNX byte representation
    ///
    /// # Arguments
    /// * `value` - The value to encode
    ///
    /// # Returns
    /// A byte array containing the encoded value
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
