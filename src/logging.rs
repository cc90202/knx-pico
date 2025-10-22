//! Unified Logging Macros for KNX-RS
//!
//! This module provides a unified logging interface that automatically
//! selects between `log::` (USB logger) and `defmt::` based on the
//! active feature flags.
//!
//! # Usage
//!
//! ```rust
//! use crate::pico_log;
//!
//! pico_log!(info, "Connection established");
//! pico_log!(debug, "Received {} bytes", n);
//! pico_log!(warn, "Timeout occurred");
//! pico_log!(error, "Failed to connect");
//! pico_log!(trace, "Entering function");
//! ```
//!
//! # Feature Flags
//!
//! - `usb-logger` - Uses `log::` crate (for USB serial debugging)
//! - No feature - Uses `defmt::` (default, more efficient for embedded)

/// Unified logging macro - automatically selects log:: or defmt:: based on features
///
/// This macro provides a consistent logging API across the entire project,
/// regardless of which logging backend is configured at compile time.
///
/// # Examples
///
/// ```rust
/// use crate::pico_log;
///
/// // Simple message
/// pico_log!(info, "System initialized");
///
/// // With formatting
/// pico_log!(debug, "Value: {}", 42);
/// pico_log!(warn, "Retry attempt {}/{}", current, max);
///
/// // Different log levels
/// pico_log!(trace, "Entering critical section");
/// pico_log!(error, "Connection failed: {}", error);
/// ```
#[macro_export]
#[cfg(feature = "usb-logger")]
macro_rules! pico_log {
    (info, $($arg:tt)*) => { log::info!($($arg)*) };
    (debug, $($arg:tt)*) => { log::debug!($($arg)*) };
    (warn, $($arg:tt)*) => { log::warn!($($arg)*) };
    (error, $($arg:tt)*) => { log::error!($($arg)*) };
    (trace, $($arg:tt)*) => { log::trace!($($arg)*) };
}

#[macro_export]
#[cfg(not(feature = "usb-logger"))]
macro_rules! pico_log {
    (info, $($arg:tt)*) => { defmt::info!($($arg)*) };
    (debug, $($arg:tt)*) => { defmt::debug!($($arg)*) };
    (warn, $($arg:tt)*) => { defmt::warn!($($arg)*) };
    (error, $($arg:tt)*) => { defmt::error!($($arg)*) };
    (trace, $($arg:tt)*) => { defmt::trace!($($arg)*) };
}

