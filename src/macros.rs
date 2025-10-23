//! Convenience macros for working with KNX addresses and types.
//!
//! This module provides declarative macros that simplify common KNX operations
//! and make code more readable and concise.

/// Creates a [`GroupAddress`](crate::addressing::GroupAddress) from 3-level notation.
///
/// The `ga!` macro provides a clean, intuitive syntax for creating group addresses
/// using the familiar KNX 3-level notation (main/middle/sub).
///
/// # Syntax
///
/// ```text
/// ga!(main/middle/sub)
/// ```
///
/// Where:
/// - `main`: Main group (0-31, typically 0-31)
/// - `middle`: Middle group (0-7)
/// - `sub`: Sub group (0-255)
///
/// # Examples
///
/// ```no_run
/// use knx_pico::ga;
///
/// // Create group address 1/2/3
/// let addr = ga!(1/2/3);
///
/// // Use in function calls
/// client.write(ga!(1/2/3), KnxValue::Bool(true)).await?;
///
/// // Multiple addresses
/// let temp_sensor = ga!(1/2/10);
/// let humidity_sensor = ga!(1/2/11);
/// let light_switch = ga!(2/1/5);
/// ```
///
/// # Compile-Time Validation
///
/// The macro validates address components at compile time:
///
/// ```compile_fail
/// // This will fail to compile: main group > 31
/// let addr = ga!(32/0/0);
/// ```
///
/// ```compile_fail
/// // This will fail to compile: middle group > 7
/// let addr = ga!(1/8/0);
/// ```
///
/// # Equivalent Code
///
/// ```rust
/// use knx_pico::addressing::GroupAddress;
///
/// // Using macro
/// let addr1 = ga!(1/2/3);
///
/// // Without macro (equivalent)
/// let addr2 = GroupAddress::from(
///     ((1u16 & 0x1F) << 11) | ((2u16 & 0x07) << 8) | (3u16 & 0xFF)
/// );
/// ```
#[macro_export]
macro_rules! ga {
    ($main:literal / $middle:literal / $sub:literal) => {{
        // Compile-time validation
        const _: () = {
            if $main > 31 {
                panic!("Main group must be 0-31");
            }
            if $middle > 7 {
                panic!("Middle group must be 0-7");
            }
            if $sub > 255 {
                panic!("Sub group must be 0-255");
            }
        };

        // Calculate raw address: MMMMMMMM MMMMSSSSSSSSS (5 bits main, 3 bits middle, 8 bits sub)
        const RAW: u16 = (($main & 0x1F) << 11) | (($middle & 0x07) << 8) | ($sub & 0xFF);
        $crate::addressing::GroupAddress::from(RAW)
    }};
}

/// Registers multiple DPT type mappings in a single block.
///
/// Simplifies batch registration of group addresses with their DPT types.
///
/// # Syntax
///
/// ```text
/// register_dpts! {
///     client,
///     main/middle/sub => DptType,
///     main/middle/sub => DptType,
///     ...
/// }
/// ```
///
/// # Examples
///
/// ```no_run
/// use knx_pico::{register_dpts, DptType};
///
/// register_dpts! {
///     client,
///     1/2/3 => Temperature,
///     1/2/4 => Bool,
///     1/2/5 => Humidity,
///     1/2/6 => Lux,
///     2/1/10 => Bool,
/// }?;
/// ```
///
/// # Error Handling
///
/// The macro returns a `Result`, so you must handle errors with `?` or `.unwrap()`:
///
/// ```no_run
/// // With ? operator (recommended)
/// register_dpts! {
///     client,
///     1/2/3 => Temperature,
/// }?;
///
/// // With unwrap (only for examples/testing)
/// register_dpts! {
///     client,
///     1/2/3 => Temperature,
/// }.unwrap();
/// ```
///
/// # Returns
///
/// - `Ok(())` - All registrations successful
/// - `Err(KnxClientError::BufferError)` - Registry is full (max 32 addresses)
///
/// # Equivalent Code
///
/// ```rust
/// use knx_pico::{ga, DptType};
///
/// // Using macro
/// register_dpts! {
///     client,
///     1/2/3 => Temperature,
///     1/2/4 => Bool,
/// }?;
///
/// // Without macro (equivalent)
/// client.register_dpt(ga!(1/2/3), DptType::Temperature)?;
/// client.register_dpt(ga!(1/2/4), DptType::Bool)?;
/// ```
#[macro_export]
macro_rules! register_dpts {
    ($client:expr, $( $main:literal / $middle:literal / $sub:literal => $dpt:ident ),* $(,)?) => {{
        use $crate::DptType;

        // Use a closure to allow early return with ?
        (|| -> $crate::knx_client::Result<()> {
            $(
                $client.register_dpt(
                    $crate::ga!($main / $middle / $sub),
                    DptType::$dpt
                )?;
            )*
            Ok(())
        })()
    }};
}

/// Simplified write operation with inline address notation.
///
/// This macro combines address creation and value writing in a single
/// convenient expression.
///
/// # Syntax
///
/// ```text
/// knx_write!(client, main/middle/sub, value)
/// ```
///
/// # Examples
///
/// ```no_run
/// use knx_pico::{knx_write, KnxValue};
///
/// // Turn on a light
/// knx_write!(client, 1/2/3, KnxValue::Bool(true)).await?;
///
/// // Set temperature
/// knx_write!(client, 1/2/10, KnxValue::Temperature(21.5)).await?;
///
/// // Set dimmer percentage
/// knx_write!(client, 2/1/5, KnxValue::Percent(75)).await?;
/// ```
///
/// # Returns
///
/// Returns the same `Result` as `KnxClient::write()`:
/// - `Ok(())` - Write successful
/// - `Err(KnxClientError)` - Write failed
///
/// # Equivalent Code
///
/// ```rust
/// use knx_pico::{ga, KnxValue};
///
/// // Using macro
/// knx_write!(client, 1/2/3, KnxValue::Bool(true)).await?;
///
/// // Without macro (equivalent)
/// client.write(ga!(1/2/3), KnxValue::Bool(true)).await?;
/// ```
#[macro_export]
macro_rules! knx_write {
    ($client:expr, $main:literal / $middle:literal / $sub:literal, $value:expr) => {
        $client.write($crate::ga!($main / $middle / $sub), $value)
    };
}

/// Simplified read operation with inline address notation.
///
/// This macro combines address creation and read request in a single
/// convenient expression.
///
/// # Syntax
///
/// ```text
/// knx_read!(client, main/middle/sub)
/// ```
///
/// # Examples
///
/// ```no_run
/// use knx_pico::knx_read;
///
/// // Request temperature value
/// knx_read!(client, 1/2/10).await?;
///
/// // Request switch state
/// knx_read!(client, 1/2/3).await?;
///
/// // Then wait for response in receive_event()
/// match client.receive_event().await? {
///     Some(KnxEvent::GroupResponse { address, value }) => {
///         // Handle response
///     }
///     _ => {}
/// }
/// ```
///
/// # Returns
///
/// Returns the same `Result` as `KnxClient::read()`:
/// - `Ok(())` - Read request sent successfully
/// - `Err(KnxClientError)` - Send failed
///
/// # Equivalent Code
///
/// ```rust
/// use knx_pico::ga;
///
/// // Using macro
/// knx_read!(client, 1/2/3).await?;
///
/// // Without macro (equivalent)
/// client.read(ga!(1/2/3)).await?;
/// ```
#[macro_export]
macro_rules! knx_read {
    ($client:expr, $main:literal / $middle:literal / $sub:literal) => {
        $client.read($crate::ga!($main / $middle / $sub))
    };
}

/// Simplified respond operation with inline address notation.
///
/// This macro combines address creation and response sending in a single
/// convenient expression.
///
/// # Syntax
///
/// ```text
/// knx_respond!(client, main/middle/sub, value)
/// ```
///
/// # Examples
///
/// ```no_run
/// use knx_pico::{knx_respond, KnxValue, KnxEvent};
///
/// // Respond to read requests
/// match client.receive_event().await? {
///     Some(KnxEvent::GroupRead { address }) => {
///         // Respond with current temperature
///         knx_respond!(client, 1/2/10, KnxValue::Temperature(21.5)).await?;
///     }
///     _ => {}
/// }
/// ```
///
/// # Returns
///
/// Returns the same `Result` as `KnxClient::respond()`:
/// - `Ok(())` - Response sent successfully
/// - `Err(KnxClientError)` - Send failed
///
/// # Equivalent Code
///
/// ```rust
/// use knx_pico::{ga, KnxValue};
///
/// // Using macro
/// knx_respond!(client, 1/2/3, KnxValue::Bool(true)).await?;
///
/// // Without macro (equivalent)
/// client.respond(ga!(1/2/3), KnxValue::Bool(true)).await?;
/// ```
#[macro_export]
macro_rules! knx_respond {
    ($client:expr, $main:literal / $middle:literal / $sub:literal, $value:expr) => {
        $client.respond($crate::ga!($main / $middle / $sub), $value)
    };
}

#[cfg(test)]
mod tests {
    use crate::addressing::GroupAddress;

    #[test]
    fn test_ga_macro_basic() {
        let addr = ga!(1 / 2 / 3);
        let expected = GroupAddress::from(0x0A03);
        assert_eq!(addr, expected);
    }

    #[test]
    fn test_ga_macro_boundaries() {
        // Test boundary values
        let addr_max = ga!(31 / 7 / 255);
        let addr_min = ga!(0 / 0 / 0);

        // Verify they compile and create valid addresses
        let _: GroupAddress = addr_max;
        let _: GroupAddress = addr_min;
    }

    #[test]
    fn test_ga_macro_various_addresses() {
        // Test common address patterns
        assert_eq!(ga!(0 / 0 / 1), GroupAddress::from(0x0001));
        assert_eq!(ga!(1 / 0 / 0), GroupAddress::from(0x0800));
        assert_eq!(ga!(5 / 3 / 100), GroupAddress::from(0x2B64));
    }
}
