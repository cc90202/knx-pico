//! Common modules shared between main.rs and examples
//!
//! These modules are hardware-specific (Raspberry Pico 2 W with Embassy)
//! and cannot be part of the public library API.

// Re-export modules from src/ so they can be accessed via common::
pub use crate::utility;
