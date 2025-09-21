//! KNX Datapoint Types (DPT)
//!
//! This module provides encoding and decoding for KNX Datapoint Types.
//! DPTs define how to interpret the data payload in KNX telegrams.
//!
//! ## Supported DPT Families
//!
//! - **DPT 1.xxx** - Boolean (1 bit): switches, buttons, binary sensors
//! - **DPT 5.xxx** - 8-bit unsigned: percentages, angles, counters (TODO)
//! - **DPT 9.xxx** - 2-byte float: temperature, illuminance, pressure (TODO)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use knx_rs::dpt::{Dpt1, DptEncode, DptDecode};
//!
//! // Encode a boolean value
//! let data = Dpt1::Switch.encode(true)?;
//!
//! // Decode back
//! let value = Dpt1::Switch.decode(&data)?;
//! assert_eq!(value, true);
//! ```

use crate::error::Result;

pub mod dpt1;

// Re-export common types
pub use dpt1::Dpt1;

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
