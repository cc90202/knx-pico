#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![expect(dead_code, reason = "Library under development, not all items used yet")]
#![doc = include_str!("../README.md")]
// Clippy configuration
#![allow(clippy::doc_markdown)] // Many KNX terms don't need backticks
#![allow(clippy::match_same_arms)] // Intentional for exhaustiveness in DPT types
#![allow(clippy::trivially_copy_pass_by_ref)] // Consistent API across DPT types
#![allow(clippy::unused_self)] // Some methods are intentionally associated
#![allow(clippy::allow_attributes_without_reason)] // Will be addressed incrementally
#![allow(clippy::undocumented_unsafe_blocks)] // Performance-critical unsafe blocks
#![allow(clippy::map_err_ignore)] // Error context not always needed
#![allow(clippy::fn_params_excessive_bools)] // Legacy API, will refactor later

//! # knx-pico
//!
//! KNXnet/IP protocol implementation for embedded systems.
//!
//! This crate provides a `no_std` implementation of the KNXnet/IP protocol,
//! designed for use with Embassy async runtime on embedded microcontrollers.
//!
//! ## Features
//!
//! - KNXnet/IP tunneling support
//! - Common Datapoint Types (DPT)
//! - Individual and Group addressing
//! - Zero-copy parsing
//! - Async/await with Embassy
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::{KnxClient, GroupAddress};
//!
//! // Connect to KNX gateway and send a command
//! let addr = GroupAddress::new(1, 2, 3).unwrap();
//! client.write_bool(addr, true).await?;
//! ```

pub mod addressing;
pub mod dpt;
pub mod error;
pub mod net;
pub mod protocol;

// Macro modules (must be declared before use)
#[macro_use]
pub mod macros;
#[macro_use]
pub mod logging;


// Re-export commonly used types
#[doc(inline)]
pub use addressing::{GroupAddress, IndividualAddress};
#[doc(inline)]
pub use dpt::{Dpt1, Dpt5, Dpt9, DptDecode, DptEncode};
#[doc(inline)]
pub use error::{KnxError, Result};
#[doc(inline)]
pub use net::Ipv4Addr;
