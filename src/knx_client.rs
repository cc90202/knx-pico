//! High-level KNX client wrapper.
//!
//! This module provides a clean API on top of [`AsyncTunnelClient`]
//! for common KNX operations with support for multiple Datapoint Types (DPT).
//!
//! # Example
//!
//! ```no_run
//! use knx_rs::addressing::GroupAddress;
//! use knx_client::{KnxClient, KnxValue};
//!
//! // Create client with builder pattern
//! let mut client = KnxClient::builder()
//!     .gateway([192, 168, 1, 10], 3671)
//!     .device_address([1, 1, 1])
//!     .build_with_buffers(&stack, &mut buffers)?;
//!
//! // Connect and use
//! client.connect().await?;
//! client.write(GroupAddress::from(0x0A03), KnxValue::Bool(true)).await?;
//! ```

use core::fmt;
use embassy_net::udp::PacketMetadata;
use heapless::index_map::FnvIndexMap;
use knx_rs::addressing::{GroupAddress, IndividualAddress};
use knx_rs::error::KnxError;
use knx_rs::protocol::async_tunnel::AsyncTunnelClient;
use knx_rs::protocol::cemi::{ControlField1, ControlField2};
use knx_rs::protocol::constants::CEMIMessageCode;

/// Default device individual address (1.1.1).
const DEVICE_ADDRESS_RAW: u16 = 0x1101;

/// Default KNXnet/IP port.
const DEFAULT_KNXNET_PORT: u16 = 3671;

/// Default buffer sizes for UDP communication.
const DEFAULT_RX_BUFFER_SIZE: usize = 2048;
const DEFAULT_TX_BUFFER_SIZE: usize = 2048;
const DEFAULT_METADATA_COUNT: usize = 4;

/// Maximum number of group addresses in DPT registry.
const MAX_DPT_REGISTRY_SIZE: usize = 32;

/// Result type for KNX client operations.
pub type Result<T> = core::result::Result<T, KnxClientError>;

/// Errors that can occur during KNX client operations.
///
/// This enum provides detailed error information for all client operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KnxClientError {
    /// Client is not connected to the gateway.
    ///
    /// Call [`KnxClient::connect()`] before performing operations.
    NotConnected,

    /// Connection to the gateway failed.
    ///
    /// This can happen during initial connection or reconnection attempts.
    ConnectionFailed(KnxError),

    /// Failed to send data to the gateway.
    ///
    /// Network or protocol error during transmission.
    SendFailed(KnxError),

    /// Failed to receive data from the gateway.
    ///
    /// Network or protocol error during reception.
    ReceiveFailed(KnxError),

    /// Operation timed out.
    ///
    /// The requested operation did not complete within the expected time.
    Timeout,

    /// Invalid or malformed address.
    InvalidAddress,

    /// Protocol error occurred.
    ///
    /// The gateway or client violated the KNX protocol specification.
    ProtocolError(KnxError),

    /// Internal buffer error.
    ///
    /// Buffer too small or other buffer-related issue.
    BufferError,

    /// Unsupported operation.
    ///
    /// The requested operation is not supported by this client or gateway.
    UnsupportedOperation,
}

impl fmt::Display for KnxClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => write!(f, "Not connected to KNX gateway"),
            Self::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            Self::SendFailed(e) => write!(f, "Send failed: {}", e),
            Self::ReceiveFailed(e) => write!(f, "Receive failed: {}", e),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::InvalidAddress => write!(f, "Invalid address"),
            Self::ProtocolError(e) => write!(f, "Protocol error: {}", e),
            Self::BufferError => write!(f, "Buffer error"),
            Self::UnsupportedOperation => write!(f, "Unsupported operation"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for KnxClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConnectionFailed(e) | Self::SendFailed(e)
            | Self::ReceiveFailed(e) | Self::ProtocolError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<KnxError> for KnxClientError {
    fn from(err: KnxError) -> Self {
        match err {
            KnxError::ConnectionFailed
            | KnxError::ConnectionRefused
            | KnxError::ConnectionTimeout
            | KnxError::ConnectionLost => Self::ConnectionFailed(err),
            KnxError::SendFailed => Self::SendFailed(err),
            KnxError::ReceiveFailed => Self::ReceiveFailed(err),
            KnxError::Timeout => Self::Timeout,
            KnxError::BufferTooSmall => Self::BufferError,
            _ => Self::ProtocolError(err),
        }
    }
}

/// Datapoint Type specification for group addresses.
///
/// Used in the DPT registry to define how values should be decoded
/// for specific group addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DptType {
    /// DPT 1.xxx - Boolean (switch, button, etc.)
    Bool,
    /// DPT 3.007 - Dimming control (increase/decrease, step size)
    Dimming,
    /// DPT 3.008 - Blinds control (up/down, step size)
    Blinds,
    /// DPT 5.001 - Percentage (0-100%)
    Percent,
    /// DPT 5.010 - Unsigned 8-bit (0-255)
    U8,
    /// DPT 7.001 - Unsigned 16-bit (0-65535)
    U16,
    /// DPT 10.001 - Time of day
    Time,
    /// DPT 11.001 - Date
    Date,
    /// DPT 16.000 / 16.001 - String ASCII (ISO 8859-1)
    StringAscii,
    /// DPT 19.001 - Date and Time
    DateTime,
    /// DPT 9.001 - Temperature in Celsius
    Temperature,
    /// DPT 9.004 - Illuminance in lux
    Lux,
    /// DPT 9.007 - Humidity percentage
    Humidity,
    /// DPT 9.008 - Air quality in ppm
    Ppm,
    /// DPT 9.xxx - Generic 2-byte float
    Float2,
}

impl DptType {
    /// Returns a human-readable name for this DPT type.
    pub const fn name(&self) -> &'static str {
        match self {
            DptType::Bool => "Boolean (DPT 1.xxx)",
            DptType::Dimming => "Dimming Control (DPT 3.007)",
            DptType::Blinds => "Blinds Control (DPT 3.008)",
            DptType::Percent => "Percentage (DPT 5.001)",
            DptType::U8 => "Unsigned 8-bit (DPT 5.010)",
            DptType::U16 => "Unsigned 16-bit (DPT 7.001)",
            DptType::Time => "Time of Day (DPT 10.001)",
            DptType::Date => "Date (DPT 11.001)",
            DptType::StringAscii => "String ASCII (DPT 16.000/16.001)",
            DptType::DateTime => "Date and Time (DPT 19.001)",
            DptType::Temperature => "Temperature (DPT 9.001)",
            DptType::Lux => "Illuminance (DPT 9.004)",
            DptType::Humidity => "Humidity (DPT 9.007)",
            DptType::Ppm => "Air Quality (DPT 9.008)",
            DptType::Float2 => "Generic Float (DPT 9.xxx)",
        }
    }

    /// Converts a decoded raw value into a typed [`KnxValue`] based on this DPT.
    fn apply_to_value(&self, raw_value: KnxValue) -> KnxValue {
        match (self, raw_value) {
            (DptType::Bool, KnxValue::Bool(v)) => KnxValue::Bool(v),
            // DPT 3.xxx: Convert U8 to Control3Bit
            (DptType::Dimming, KnxValue::U8(v)) | (DptType::Blinds, KnxValue::U8(v)) => {
                let control = (v & 0x08) != 0; // Bit 3
                let step = v & 0x07; // Bits 2-0
                KnxValue::Control3Bit { control, step }
            }
            // DPT 3.xxx: Already decoded
            (DptType::Dimming, KnxValue::Control3Bit { control, step })
            | (DptType::Blinds, KnxValue::Control3Bit { control, step }) => {
                KnxValue::Control3Bit { control, step }
            }
            (DptType::Percent, KnxValue::U8(v)) => KnxValue::Percent(v.min(100)),
            (DptType::U8, KnxValue::U8(v)) => KnxValue::U8(v),
            (DptType::U16, KnxValue::U16(v)) => KnxValue::U16(v),
            // DPT 10.001: Already decoded as Time variant
            (DptType::Time, KnxValue::Time { day, hour, minute, second }) => {
                KnxValue::Time { day, hour, minute, second }
            }
            // DPT 11.001: Already decoded as Date variant
            (DptType::Date, KnxValue::Date { day, month, year }) => {
                KnxValue::Date { day, month, year }
            }
            // DPT 16.xxx: Already decoded as StringAscii variant
            (DptType::StringAscii, KnxValue::StringAscii { data, len }) => {
                KnxValue::StringAscii { data, len }
            }
            // DPT 19.001: Already decoded as DateTime variant
            (DptType::DateTime, val @ KnxValue::DateTime { .. }) => val,
            (DptType::Temperature, KnxValue::Float2(v)) => KnxValue::Temperature(v),
            (DptType::Lux, KnxValue::Float2(v)) => KnxValue::Lux(v),
            (DptType::Humidity, KnxValue::Float2(v)) => KnxValue::Humidity(v),
            (DptType::Ppm, KnxValue::Float2(v)) => KnxValue::Ppm(v),
            (DptType::Float2, KnxValue::Float2(v)) => KnxValue::Float2(v),
            // Fallback: return the raw value if types don't match
            _ => raw_value,
        }
    }
}

/// KNX value types representing different Datapoint Types (DPT).
///
/// This enum provides type-safe representations of common KNX datapoint types.
/// Each variant corresponds to a specific DPT specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KnxValue {
    /// Boolean value (DPT 1.xxx) - switch, enable/disable
    Bool(bool),
    /// 3-bit controlled value (DPT 3.xxx) - dimming, blinds
    ///
    /// Format: 1 byte with control bit and step code
    /// - `control`: Direction (false = decrease/up, true = increase/down)
    /// - `step`: Step code (0 = break/stop, 1-7 = step sizes)
    Control3Bit {
        /// Control direction: false = decrease/up, true = increase/down
        control: bool,
        /// Step code: 0 = break, 1-7 = step size
        step: u8,
    },
    /// Percentage value (DPT 5.001) - 0-100%
    Percent(u8),
    /// Unsigned 8-bit value (DPT 5.010) - 0-255
    U8(u8),
    /// Unsigned 16-bit value (DPT 7.001) - 0-65535, counters, pulses
    U16(u16),
    /// Time of day (DPT 10.001)
    ///
    /// Format: 3 bytes encoding day, hour, minute, second
    /// - `day`: 0 = no day, 1 = Monday, ..., 7 = Sunday
    /// - `hour`: 0-23
    /// - `minute`: 0-59
    /// - `second`: 0-59
    Time {
        /// Day of week (0 = no day, 1 = Monday, ..., 7 = Sunday)
        day: u8,
        /// Hour (0-23)
        hour: u8,
        /// Minute (0-59)
        minute: u8,
        /// Second (0-59)
        second: u8,
    },
    /// Date (DPT 11.001)
    ///
    /// Format: 3 bytes encoding day, month, year
    /// - `day`: 1-31 (day of month)
    /// - `month`: 1-12
    /// - `year`: 0-99 (2000-2099) or 1990-2089 depending on implementation
    Date {
        /// Day of month (1-31)
        day: u8,
        /// Month (1-12)
        month: u8,
        /// Year (0-99, representing 1990-2089 or 2000-2099)
        year: u8,
    },
    /// String ASCII (DPT 16.000/16.001)
    ///
    /// Format: Up to 14 ASCII characters (ISO 8859-1)
    /// - Null-terminated or padded with null bytes
    /// - Maximum 14 characters
    StringAscii {
        /// String data (up to 14 bytes, null-terminated)
        data: [u8; 14],
        /// Actual string length (excluding null terminator)
        len: u8,
    },
    /// Date and Time (DPT 19.001)
    ///
    /// Format: 8 bytes encoding year, month, day, day of week, hour, minute, second, flags
    /// - `year`: 1900-2155 (full year)
    /// - `month`: 1-12
    /// - `day`: 1-31
    /// - `day_of_week`: 0 = no day, 1 = Monday, ..., 7 = Sunday
    /// - `hour`: 0-23
    /// - `minute`: 0-59
    /// - `second`: 0-59
    /// - `fault`: Fault flag
    /// - `working_day`: Working day flag
    /// - `no_wd`: No working day valid flag
    /// - `no_year`: No year valid flag
    /// - `no_date`: No date valid flag
    /// - `no_dow`: No day of week valid flag
    /// - `no_time`: No time valid flag
    /// - `standard_summertime`: Standard/summertime flag (false = standard, true = summertime)
    /// - `quality`: Clock quality flag
    DateTime {
        /// Year (1900-2155)
        year: u16,
        /// Month (1-12)
        month: u8,
        /// Day of month (1-31)
        day: u8,
        /// Day of week (0 = no day, 1 = Monday, ..., 7 = Sunday)
        day_of_week: u8,
        /// Hour (0-23)
        hour: u8,
        /// Minute (0-59)
        minute: u8,
        /// Second (0-59)
        second: u8,
        /// Fault flag
        fault: bool,
        /// Working day flag
        working_day: bool,
        /// No working day valid flag
        no_wd: bool,
        /// No year valid flag
        no_year: bool,
        /// No date valid flag
        no_date: bool,
        /// No day of week valid flag
        no_dow: bool,
        /// No time valid flag
        no_time: bool,
        /// Standard/summertime flag (false = standard, true = summertime)
        standard_summertime: bool,
        /// Clock quality flag
        quality: bool,
    },
    /// Temperature in Celsius (DPT 9.001)
    Temperature(f32),
    /// Illuminance in lux (DPT 9.004)
    Lux(f32),
    /// Humidity percentage (DPT 9.007) - 0-100%
    Humidity(f32),
    /// Air quality in parts per million (DPT 9.008)
    Ppm(f32),
    /// Generic 2-byte float (DPT 9.xxx) for other variants
    Float2(f32),
}

/// Events received from the KNX bus.
///
/// Represents parsed KNX bus events with typed values.
#[derive(Debug)]
pub enum KnxEvent {
    /// Group value write event - a device wrote a value to the bus
    GroupWrite {
        /// Destination group address
        address: GroupAddress,
        /// Value that was written
        value: KnxValue,
    },
    /// Group value read request - a device requested a value
    GroupRead {
        /// Group address being queried
        address: GroupAddress,
    },
    /// Group value response - answer to a read request
    GroupResponse {
        /// Group address responding
        address: GroupAddress,
        /// Response value
        value: KnxValue,
    },
    /// Unknown or unparsed event
    Unknown {
        /// Group address
        address: GroupAddress,
        /// Data length in bytes
        data_len: usize,
    },
}

/// Buffer storage for KNX client.
///
/// This struct holds all the buffers needed for UDP communication.
/// Use this with [`KnxClientBuilder::build_with_buffers`].
pub struct KnxBuffers {
    /// Receive packet metadata
    pub rx_meta: [PacketMetadata; DEFAULT_METADATA_COUNT],
    /// Transmit packet metadata
    pub tx_meta: [PacketMetadata; DEFAULT_METADATA_COUNT],
    /// Receive data buffer
    pub rx_buffer: [u8; DEFAULT_RX_BUFFER_SIZE],
    /// Transmit data buffer
    pub tx_buffer: [u8; DEFAULT_TX_BUFFER_SIZE],
}

impl KnxBuffers {
    /// Creates a new buffer storage with default sizes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut buffers = KnxBuffers::new();
    /// let client = KnxClient::builder()
    ///     .gateway([192, 168, 1, 10], 3671)
    ///     .build_with_buffers(&stack, &mut buffers)?;
    /// ```
    pub const fn new() -> Self {
        Self {
            rx_meta: [PacketMetadata::EMPTY; DEFAULT_METADATA_COUNT],
            tx_meta: [PacketMetadata::EMPTY; DEFAULT_METADATA_COUNT],
            rx_buffer: [0u8; DEFAULT_RX_BUFFER_SIZE],
            tx_buffer: [0u8; DEFAULT_TX_BUFFER_SIZE],
        }
    }
}

impl Default for KnxBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring and creating a [`KnxClient`].
///
/// # Example
///
/// ```no_run
/// use knx_client::{KnxClient, KnxBuffers};
///
/// let mut buffers = KnxBuffers::new();
/// let mut client = KnxClient::builder()
///     .gateway([192, 168, 1, 10], 3671)
///     .device_address([1, 1, 1])
///     .build_with_buffers(&stack, &mut buffers)?;
///
/// client.connect().await?;
/// ```
pub struct KnxClientBuilder {
    gateway_ip: [u8; 4],
    gateway_port: u16,
    device_address: u16,
}

impl KnxClientBuilder {
    /// Creates a new builder with default values.
    ///
    /// Default configuration:
    /// - Gateway: `[192, 168, 1, 10]:3671`
    /// - Device address: `1.1.1`
    fn new() -> Self {
        Self {
            gateway_ip: [192, 168, 1, 10],
            gateway_port: DEFAULT_KNXNET_PORT,
            device_address: DEVICE_ADDRESS_RAW,
        }
    }

    /// Sets the KNX gateway IP address and port.
    ///
    /// # Arguments
    ///
    /// * `ip` - Gateway IP address as `[u8; 4]` (e.g., `[192, 168, 1, 10]`)
    /// * `port` - Gateway port (typically 3671)
    ///
    /// # Example
    ///
    /// ```no_run
    /// let builder = KnxClient::builder()
    ///     .gateway([192, 168, 1, 10], 3671);
    /// ```
    pub fn gateway(mut self, ip: [u8; 4], port: u16) -> Self {
        self.gateway_ip = ip;
        self.gateway_port = port;
        self
    }

    /// Sets the device individual address.
    ///
    /// # Arguments
    ///
    /// * `address` - Device address as `[area, line, device]`
    ///   For example: `[1, 1, 1]` represents address `1.1.1`
    ///
    /// # Example
    ///
    /// ```no_run
    /// let builder = KnxClient::builder()
    ///     .device_address([1, 1, 250]);  // 1.1.250
    /// ```
    pub fn device_address(mut self, address: [u8; 3]) -> Self {
        let [area, line, device] = address;
        self.device_address = ((area as u16) << 12) | ((line as u16) << 8) | (device as u16);
        self
    }

    /// Builds the [`KnxClient`] using provided buffers.
    ///
    /// This method gives you control over buffer allocation.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack reference
    /// * `buffers` - Mutable reference to [`KnxBuffers`]
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut buffers = KnxBuffers::new();
    /// let client = KnxClient::builder()
    ///     .gateway([192, 168, 1, 10], 3671)
    ///     .build_with_buffers(&stack, &mut buffers)?;
    /// ```
    pub fn build_with_buffers<'a>(
        self,
        stack: &'a embassy_net::Stack<'static>,
        buffers: &'a mut KnxBuffers,
    ) -> Result<KnxClient<'a>> {
        Ok(KnxClient::new_with_device(
            stack,
            &mut buffers.rx_meta,
            &mut buffers.tx_meta,
            &mut buffers.rx_buffer,
            &mut buffers.tx_buffer,
            self.gateway_ip,
            self.gateway_port,
            self.device_address,
        ))
    }

    /// Builds the [`KnxClient`] using custom buffers.
    ///
    /// This is a lower-level method for advanced users who want full control
    /// over buffer sizes and allocation.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack reference
    /// * `rx_meta` - Receive packet metadata buffer (minimum 4 entries)
    /// * `tx_meta` - Transmit packet metadata buffer (minimum 4 entries)
    /// * `rx_buffer` - Receive data buffer (recommended 2048 bytes)
    /// * `tx_buffer` - Transmit data buffer (recommended 2048 bytes)
    pub fn build<'a>(
        self,
        stack: &'a embassy_net::Stack<'static>,
        rx_meta: &'a mut [PacketMetadata],
        tx_meta: &'a mut [PacketMetadata],
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
    ) -> Result<KnxClient<'a>> {
        Ok(KnxClient::new_with_device(
            stack,
            rx_meta,
            tx_meta,
            rx_buffer,
            tx_buffer,
            self.gateway_ip,
            self.gateway_port,
            self.device_address,
        ))
    }
}

/// High-level KNX client for tunneling operations.
///
/// Provides a simplified async API for KNX operations including
/// write, read, respond, and event receiving.
///
/// # Creating a Client
///
/// Use the builder pattern for easy configuration:
///
/// ```no_run
/// use knx_client::{KnxClient, KnxBuffers};
///
/// let mut buffers = KnxBuffers::new();
/// let mut client = KnxClient::builder()
///     .gateway([192, 168, 1, 10], 3671)
///     .device_address([1, 1, 1])
///     .build_with_buffers(&stack, &mut buffers)?;
/// ```
pub struct KnxClient<'a> {
    tunnel: AsyncTunnelClient<'a>,
    device_address: u16,
    /// DPT registry mapping group addresses to their datapoint types
    dpt_registry: FnvIndexMap<u16, DptType, MAX_DPT_REGISTRY_SIZE>,
}

impl<'a> KnxClient<'a> {
    /// Creates a builder for configuring a new KNX client.
    ///
    /// This is the recommended way to create a [`KnxClient`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use knx_client::{KnxClient, KnxBuffers};
    ///
    /// let mut buffers = KnxBuffers::new();
    /// let mut client = KnxClient::builder()
    ///     .gateway([192, 168, 1, 10], 3671)
    ///     .device_address([1, 1, 1])
    ///     .build_with_buffers(&stack, &mut buffers)?;
    /// ```
    pub fn builder() -> KnxClientBuilder {
        KnxClientBuilder::new()
    }

    /// Creates a new KNX client instance with custom device address.
    ///
    /// Internal method used by the builder. Prefer using [`KnxClient::builder()`].
    fn new_with_device(
        stack: &'a embassy_net::Stack<'static>,
        rx_meta: &'a mut [PacketMetadata],
        tx_meta: &'a mut [PacketMetadata],
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
        gateway_ip: [u8; 4],
        gateway_port: u16,
        device_address: u16,
    ) -> Self {
        let tunnel = AsyncTunnelClient::new(
            stack,
            rx_meta,
            tx_meta,
            rx_buffer,
            tx_buffer,
            gateway_ip,
            gateway_port,
        );

        Self {
            tunnel,
            device_address,
            dpt_registry: FnvIndexMap::new(),
        }
    }

    /// Creates a new KNX client instance.
    ///
    /// **Note:** Consider using [`KnxClient::builder()`] for a more ergonomic API.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack reference
    /// * `rx_meta` - Receive packet metadata buffer (minimum 4 entries)
    /// * `tx_meta` - Transmit packet metadata buffer (minimum 4 entries)
    /// * `rx_buffer` - Receive data buffer (recommended 2048 bytes)
    /// * `tx_buffer` - Transmit data buffer (recommended 2048 bytes)
    /// * `gateway_ip` - KNX gateway IP address as `[u8; 4]`
    /// * `gateway_port` - KNX gateway port (typically 3671)
    ///
    /// # Returns
    ///
    /// New `KnxClient` instance ready to connect.
    pub fn new(
        stack: &'a embassy_net::Stack<'static>,
        rx_meta: &'a mut [PacketMetadata],
        tx_meta: &'a mut [PacketMetadata],
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
        gateway_ip: [u8; 4],
        gateway_port: u16,
    ) -> Self {
        Self::new_with_device(
            stack,
            rx_meta,
            tx_meta,
            rx_buffer,
            tx_buffer,
            gateway_ip,
            gateway_port,
            DEVICE_ADDRESS_RAW,
        )
    }

    /// Registers a datapoint type for a specific group address.
    ///
    /// This allows automatic type conversion when receiving events for this address.
    /// Registered addresses will have their values decoded according to the DPT type.
    ///
    /// # Arguments
    ///
    /// * `address` - Group address to register
    /// * `dpt_type` - Datapoint type specification
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Registration successful
    /// * `Err(KnxClientError::BufferError)` - Registry is full (max 32 addresses)
    ///
    /// # Example
    ///
    /// ```no_run
    /// // Register address 1/2/3 as a temperature sensor
    /// client.register_dpt(
    ///     GroupAddress::from(0x0A03),
    ///     DptType::Temperature
    /// )?;
    ///
    /// // Register address 1/2/4 as a light switch
    /// client.register_dpt(
    ///     GroupAddress::from(0x0A04),
    ///     DptType::Bool
    /// )?;
    ///
    /// // Now receive_event() will automatically apply the correct types
    /// ```
    pub fn register_dpt(&mut self, address: GroupAddress, dpt_type: DptType) -> Result<()> {
        let addr_raw: u16 = address.into();
        self.dpt_registry
            .insert(addr_raw, dpt_type)
            .map_err(|_| KnxClientError::BufferError)?;
        Ok(())
    }

    /// Looks up the registered DPT type for a group address.
    ///
    /// # Arguments
    ///
    /// * `address` - Group address to look up
    ///
    /// # Returns
    ///
    /// * `Some(DptType)` - Registered DPT type for this address
    /// * `None` - Address not registered
    pub fn lookup_dpt(&self, address: GroupAddress) -> Option<DptType> {
        let addr_raw: u16 = address.into();
        self.dpt_registry.get(&addr_raw).copied()
    }

    /// Clears all registered DPT mappings.
    pub fn clear_dpt_registry(&mut self) {
        self.dpt_registry.clear();
    }

    /// Returns the number of registered DPT mappings.
    pub fn dpt_registry_len(&self) -> usize {
        self.dpt_registry.len()
    }

    /// Returns whether the DPT registry is empty.
    pub fn dpt_registry_is_empty(&self) -> bool {
        self.dpt_registry.is_empty()
    }

    /// Establishes connection to the KNX gateway.
    ///
    /// Must be called before any other operations.
    ///
    /// # Errors
    ///
    /// Returns [`KnxClientError::ConnectionFailed`] if connection fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// match client.connect().await {
    ///     Ok(()) => println!("Connected!"),
    ///     Err(e) => eprintln!("Connection failed: {}", e),
    /// }
    /// ```
    pub async fn connect(&mut self) -> Result<()> {
        self.tunnel.connect().await.map_err(KnxClientError::from)
    }

    /// Writes a value to a group address (GroupValue_Write).
    ///
    /// # Arguments
    ///
    /// * `address` - Target group address
    /// * `value` - Value to write
    ///
    /// # Errors
    ///
    /// - [`KnxClientError::NotConnected`] - Client not connected
    /// - [`KnxClientError::SendFailed`] - Failed to send telegram
    ///
    /// # Example
    ///
    /// ```no_run
    /// client.write(
    ///     GroupAddress::from(0x0A03),
    ///     KnxValue::Bool(true)
    /// ).await?;
    /// ```
    pub async fn write(&mut self, address: GroupAddress, value: KnxValue) -> Result<()> {
        let mut buffer = [0u8; 16];
        let len = build_group_write(address, value, self.device_address, &mut buffer);
        self.tunnel
            .send_cemi(&buffer[..len])
            .await
            .map_err(KnxClientError::from)
    }

    /// Requests to read a value from a group address (GroupValue_Read).
    ///
    /// Other devices on the bus may respond with a [`KnxEvent::GroupResponse`].
    ///
    /// # Arguments
    ///
    /// * `address` - Group address to read from
    ///
    /// # Errors
    ///
    /// - [`KnxClientError::NotConnected`] - Client not connected
    /// - [`KnxClientError::SendFailed`] - Failed to send read request
    ///
    /// # Example
    ///
    /// ```no_run
    /// // Request temperature value
    /// client.read(GroupAddress::from(0x0A03)).await?;
    ///
    /// // Wait for response in receive_event()
    /// ```
    pub async fn read(&mut self, address: GroupAddress) -> Result<()> {
        let cemi = build_group_read(address, self.device_address);
        self.tunnel.send_cemi(&cemi).await.map_err(KnxClientError::from)
    }

    /// Responds with a value to a group address (GroupValue_Response).
    ///
    /// Typically used to answer a [`KnxEvent::GroupRead`] request.
    ///
    /// # Arguments
    ///
    /// * `address` - Group address to respond to
    /// * `value` - Response value
    ///
    /// # Errors
    ///
    /// - [`KnxClientError::NotConnected`] - Client not connected
    /// - [`KnxClientError::SendFailed`] - Failed to send response
    ///
    /// # Example
    ///
    /// ```no_run
    /// // Respond to a read request
    /// match client.receive_event().await? {
    ///     Some(KnxEvent::GroupRead { address }) => {
    ///         client.respond(address, KnxValue::Temperature(21.5)).await?;
    ///     }
    ///     _ => {}
    /// }
    /// ```
    pub async fn respond(&mut self, address: GroupAddress, value: KnxValue) -> Result<()> {
        let mut buffer = [0u8; 16];
        let len = build_group_response(address, value, self.device_address, &mut buffer);
        self.tunnel
            .send_cemi(&buffer[..len])
            .await
            .map_err(KnxClientError::from)
    }

    /// Sends a raw cEMI frame (for advanced usage).
    ///
    /// # Arguments
    ///
    /// * `cemi` - Raw cEMI frame bytes
    ///
    /// # Errors
    ///
    /// - [`KnxClientError::NotConnected`] - Client not connected
    /// - [`KnxClientError::SendFailed`] - Failed to send frame
    pub async fn send_raw_cemi(&mut self, cemi: &[u8]) -> Result<()> {
        self.tunnel.send_cemi(cemi).await.map_err(KnxClientError::from)
    }

    /// Waits for and parses the next KNX bus event.
    ///
    /// Returns `Ok(None)` on timeout (no data available).
    ///
    /// # Returns
    ///
    /// * `Ok(Some(event))` - Parsed KNX event
    /// * `Ok(None)` - Timeout, no data available
    ///
    /// # Errors
    ///
    /// - [`KnxClientError::NotConnected`] - Client not connected
    /// - [`KnxClientError::ReceiveFailed`] - Failed to receive data
    /// - [`KnxClientError::ProtocolError`] - Invalid or malformed frame
    ///
    /// # Example
    ///
    /// ```no_run
    /// loop {
    ///     match client.receive_event().await? {
    ///         Some(KnxEvent::GroupWrite { address, value }) => {
    ///             println!("Received write to {}: {:?}", address, value);
    ///         }
    ///         Some(KnxEvent::GroupRead { address }) => {
    ///             println!("Received read request for {}", address);
    ///         }
    ///         None => {
    ///             // Timeout, no data
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub async fn receive_event(&mut self) -> Result<Option<KnxEvent>> {
        match self.tunnel.receive().await {
            Ok(Some(cemi_data)) => {
                if let Ok(cemi) = knx_rs::protocol::cemi::CEMIFrame::parse(cemi_data) {
                    if let Ok(ldata) = cemi.as_ldata() {
                        if let Some(dest) = ldata.destination_group() {
                            if !ldata.data.is_empty() {
                                let apci = ldata.data[0] & 0xC0;

                                if apci == 0x80 {
                                    if let Some(value) = decode_value(ldata.data) {
                                        // Apply DPT type from registry if registered
                                        let typed_value = if let Some(dpt_type) = self.lookup_dpt(dest) {
                                            dpt_type.apply_to_value(value)
                                        } else {
                                            value
                                        };

                                        return Ok(Some(KnxEvent::GroupWrite {
                                            address: dest,
                                            value: typed_value,
                                        }));
                                    } else {
                                        return Ok(Some(KnxEvent::Unknown {
                                            address: dest,
                                            data_len: ldata.data.len(),
                                        }));
                                    }
                                } else if apci == 0x40 {
                                    if let Some(value) = decode_value(ldata.data) {
                                        // Apply DPT type from registry if registered
                                        let typed_value = if let Some(dpt_type) = self.lookup_dpt(dest) {
                                            dpt_type.apply_to_value(value)
                                        } else {
                                            value
                                        };

                                        return Ok(Some(KnxEvent::GroupResponse {
                                            address: dest,
                                            value: typed_value,
                                        }));
                                    } else {
                                        return Ok(Some(KnxEvent::Unknown {
                                            address: dest,
                                            data_len: ldata.data.len(),
                                        }));
                                    }
                                } else if apci == 0x00 {
                                    return Ok(Some(KnxEvent::GroupRead { address: dest }));
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Ok(None) => Ok(None),
            Err(e) => Err(KnxClientError::from(e)),
        }
    }
}

/// Builds a cEMI L_Data.req frame for GroupValue_Write.
///
/// # Arguments
///
/// * `group_addr` - Destination group address
/// * `value` - Value to encode
/// * `device_address` - Source device address (raw u16)
/// * `buffer` - Output buffer (minimum 16 bytes)
///
/// # Returns
///
/// Total frame length in bytes.
fn build_group_write(
    group_addr: GroupAddress,
    value: KnxValue,
    device_address: u16,
    buffer: &mut [u8],
) -> usize {
    let device_addr = IndividualAddress::from(device_address);

    buffer[0] = CEMIMessageCode::LDataReq.to_u8();
    buffer[1] = 0x00; // Additional info length

    buffer[2] = ControlField1::default().raw();
    buffer[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    buffer[4] = source_bytes[0];
    buffer[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    buffer[6] = dest_bytes[0];
    buffer[7] = dest_bytes[1];

    buffer[9] = 0x00; // TPCI/APCI

    encode_value(value, &mut buffer[8..])
}

/// Builds a cEMI L_Data.req frame for GroupValue_Response.
///
/// # Arguments
///
/// * `group_addr` - Destination group address
/// * `value` - Value to encode
/// * `device_address` - Source device address (raw u16)
/// * `buffer` - Output buffer (minimum 16 bytes)
///
/// # Returns
///
/// Total frame length in bytes.
fn build_group_response(
    group_addr: GroupAddress,
    value: KnxValue,
    device_address: u16,
    buffer: &mut [u8],
) -> usize {
    let device_addr = IndividualAddress::from(device_address);

    buffer[0] = CEMIMessageCode::LDataReq.to_u8();
    buffer[1] = 0x00;

    buffer[2] = ControlField1::default().raw();
    buffer[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    buffer[4] = source_bytes[0];
    buffer[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    buffer[6] = dest_bytes[0];
    buffer[7] = dest_bytes[1];

    buffer[9] = 0x00;

    encode_value_response(value, &mut buffer[8..])
}

/// Builds a cEMI L_Data.req frame for GroupValue_Read.
///
/// # Arguments
///
/// * `group_addr` - Group address to query
/// * `device_address` - Source device address (raw u16)
///
/// # Returns
///
/// Fixed-size 11-byte cEMI frame.
fn build_group_read(group_addr: GroupAddress, device_address: u16) -> [u8; 11] {
    let mut frame = [0u8; 11];
    let device_addr = IndividualAddress::from(device_address);

    frame[0] = CEMIMessageCode::LDataReq.to_u8();
    frame[1] = 0x00;

    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();

    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    frame[8] = 0x01; // NPDU length
    frame[9] = 0x00; // TPCI/APCI
    frame[10] = 0x00; // APCI only (no data)

    frame
}

/// Encodes [`KnxValue`] to NPDU + TPCI/APCI + data for Write operation.
///
/// Buffer should start at byte 8 (NPDU length position).
///
/// # Returns
///
/// Total frame length in bytes.
fn encode_value(value: KnxValue, buffer: &mut [u8]) -> usize {
    encode_value_with_apci(value, buffer, 0x80)
}

/// Encodes [`KnxValue`] to NPDU + TPCI/APCI + data for Response operation.
///
/// Buffer should start at byte 8 (NPDU length position).
///
/// # Returns
///
/// Total frame length in bytes.
fn encode_value_response(value: KnxValue, buffer: &mut [u8]) -> usize {
    encode_value_with_apci(value, buffer, 0x40)
}

/// Encodes [`KnxValue`] with specified APCI code.
///
/// # Arguments
///
/// * `value` - Value to encode
/// * `buffer` - Output buffer starting at NPDU position
/// * `apci` - APCI code (0x80 for Write, 0x40 for Response)
///
/// # Returns
///
/// Total frame length in bytes.
fn encode_value_with_apci(value: KnxValue, buffer: &mut [u8], apci: u8) -> usize {
    match value {
        KnxValue::Bool(b) => {
            buffer[0] = 0x01;
            buffer[1] = 0x00;
            buffer[2] = apci | if b { 0x01 } else { 0x00 };
            11
        }
        KnxValue::Control3Bit { control, step } => {
            // DPT 3.xxx: 1 byte encoding
            // Bit 3: control (0 = decrease/up, 1 = increase/down)
            // Bits 2-0: step code (0-7)
            let step = step.min(7); // Clamp to 0-7
            let value = if control {
                0x08 | step // Set bit 3 for increase/down
            } else {
                step // Clear bit 3 for decrease/up
            };
            buffer[0] = 0x02; // NPDU length
            buffer[1] = 0x00; // TPCI
            buffer[2] = apci;
            buffer[3] = value;
            12
        }
        KnxValue::Percent(p) => {
            let value = ((p.min(100) as u16 * 255) / 100) as u8;
            buffer[0] = 0x02;
            buffer[1] = 0x00;
            buffer[2] = apci;
            buffer[3] = value;
            12
        }
        KnxValue::U8(v) => {
            buffer[0] = 0x02;
            buffer[1] = 0x00;
            buffer[2] = apci;
            buffer[3] = v;
            12
        }
        KnxValue::U16(v) => {
            buffer[0] = 0x03;
            buffer[1] = 0x00;
            buffer[2] = apci;
            buffer[3] = (v >> 8) as u8;
            buffer[4] = (v & 0xFF) as u8;
            13
        }
        KnxValue::Time { day, hour, minute, second } => {
            // DPT 10.001: 3 bytes encoding day, hour, minute, second
            // Byte 1: (day<<5) | hour
            // Byte 2: minute
            // Byte 3: second
            let day = day.min(7);       // Clamp to 0-7
            let hour = hour.min(23);     // Clamp to 0-23
            let minute = minute.min(59); // Clamp to 0-59
            let second = second.min(59); // Clamp to 0-59

            buffer[0] = 0x04; // NPDU length (4 bytes including TPCI/APCI)
            buffer[1] = 0x00; // TPCI
            buffer[2] = apci;
            buffer[3] = (day << 5) | hour;
            buffer[4] = minute;
            buffer[5] = second;
            14
        }
        KnxValue::Date { day, month, year } => {
            // DPT 11.001: 3 bytes encoding day, month, year
            // Byte 1: day (1-31)
            // Byte 2: month (1-12)
            // Byte 3: year (0-99)
            let day = day.max(1).min(31);   // Clamp to 1-31
            let month = month.max(1).min(12); // Clamp to 1-12
            let year = year.min(99);         // Clamp to 0-99

            buffer[0] = 0x04; // NPDU length (4 bytes including TPCI/APCI)
            buffer[1] = 0x00; // TPCI
            buffer[2] = apci;
            buffer[3] = day;
            buffer[4] = month;
            buffer[5] = year;
            14
        }
        KnxValue::StringAscii { data, len } => {
            // DPT 16.000/16.001: 14 bytes ASCII string
            // Null-terminated or padded with nulls
            let len = len.min(14); // Clamp to 0-14

            buffer[0] = 0x0F; // NPDU length (15 bytes: TPCI/APCI + 14 data bytes)
            buffer[1] = 0x00; // TPCI
            buffer[2] = apci;

            // Copy string data
            for i in 0..14 {
                if i < len as usize {
                    buffer[3 + i] = data[i];
                } else {
                    buffer[3 + i] = 0; // Null padding
                }
            }
            25 // Total frame length: 11 (cEMI header) + 14 (string data)
        }
        KnxValue::DateTime {
            year,
            month,
            day,
            day_of_week,
            hour,
            minute,
            second,
            fault,
            working_day,
            no_wd,
            no_year,
            no_date,
            no_dow,
            no_time,
            standard_summertime,
            quality,
        } => {
            // DPT 19.001: 8 bytes encoding date/time with flags
            let year = year.clamp(1900, 2155) - 1900; // Year since 1900
            let month = month.max(1).min(12);
            let day = day.max(1).min(31);
            let day_of_week = day_of_week.min(7);
            let hour = hour.min(23);
            let minute = minute.min(59);
            let second = second.min(59);

            buffer[0] = 0x09; // NPDU length (9 bytes: TPCI/APCI + 8 data bytes)
            buffer[1] = 0x00; // TPCI
            buffer[2] = apci;
            buffer[3] = year as u8;
            buffer[4] = month;
            buffer[5] = day;
            buffer[6] = (day_of_week << 5) | hour;
            buffer[7] = minute;
            buffer[8] = second;

            // Flags byte 1 (byte 9)
            let mut flags1 = 0u8;
            if fault { flags1 |= 0x80; }
            if working_day { flags1 |= 0x40; }
            if no_wd { flags1 |= 0x20; }
            if no_year { flags1 |= 0x10; }
            if no_date { flags1 |= 0x08; }
            if no_dow { flags1 |= 0x04; }
            if no_time { flags1 |= 0x02; }
            if standard_summertime { flags1 |= 0x01; }
            buffer[9] = flags1;

            // Flags byte 2 (byte 10) - only bit 7 is used for quality
            let flags2 = if quality { 0x80 } else { 0x00 };
            buffer[10] = flags2;

            19 // Total frame length: 11 (cEMI header) + 8 (data)
        }
        KnxValue::Temperature(t) | KnxValue::Lux(t) | KnxValue::Humidity(t)
        | KnxValue::Ppm(t) | KnxValue::Float2(t) => {
            let encoded = encode_dpt9(t);
            buffer[0] = 0x03;
            buffer[1] = 0x00;
            buffer[2] = apci;
            buffer[3] = (encoded >> 8) as u8;
            buffer[4] = (encoded & 0xFF) as u8;
            13
        }
    }
}

/// Encodes f32 to DPT 9.001 (2-byte float) format.
///
/// Uses KNX-specific encoding: `(0.01 * mantissa) * 2^exponent`.
///
/// # Arguments
///
/// * `value` - Float value to encode
///
/// # Returns
///
/// 16-bit encoded value in format: MEEE EMMM MMMM MMMM
/// where M is sign bit, E is 4-bit exponent, M is 11-bit mantissa.
fn encode_dpt9(value: f32) -> u16 {
    let value = value.clamp(-671_088.6, 670_760.96);

    let mut exponent = 0i32;
    let mut mantissa = (value * 100.0) as i32;

    while !(-2048..=2047).contains(&mantissa) {
        mantissa >>= 1;
        exponent += 1;
    }

    exponent = exponent.clamp(0, 15);

    let sign = if mantissa < 0 { 1u16 << 15 } else { 0 };
    let exp_bits = ((exponent as u16) & 0x0F) << 11;
    let mantissa_bits = mantissa.unsigned_abs() as u16 & 0x07FF;

    sign | exp_bits | mantissa_bits
}

/// Decodes APCI+data bytes to [`KnxValue`].
///
/// # Note
///
/// Cannot distinguish between variants with the same encoding:
/// - 1-byte data: Returns [`KnxValue::U8`] (could also be `Percent` or `Control3Bit`)
/// - 2-byte unsigned: Returns [`KnxValue::U16`]
/// - 2-byte float: Returns [`KnxValue::Float2`] (could be `Temperature`, `Lux`, etc.)
///
/// Application should interpret based on group address context using DPT registry.
///
/// # Arguments
///
/// * `data` - APCI + data bytes
///
/// # Returns
///
/// Decoded [`KnxValue`] or `None` if decoding fails.
fn decode_value(data: &[u8]) -> Option<KnxValue> {
    match data.len() {
        1 => {
            let value = (data[0] & 0x01) != 0;
            Some(KnxValue::Bool(value))
        }
        2 => {
            let raw = data[1];
            Some(KnxValue::U8(raw))
        }
        3 => {
            let byte1 = data[1];
            let byte2 = data[2];
            let raw_u16 = ((byte1 as u16) << 8) | (byte2 as u16);

            if (raw_u16 & 0x8000) != 0 || (raw_u16 & 0x7800) != 0 {
                let value = decode_dpt9(raw_u16);
                Some(KnxValue::Float2(value))
            } else {
                Some(KnxValue::U16(raw_u16))
            }
        }
        4 => {
            // Ambiguous: could be DPT 10.001 (Time) or DPT 11.001 (Date)
            // Use heuristic: if byte1 <= 31 and byte2 <= 12 and byte3 <= 99, assume Date
            // Otherwise assume Time
            let byte1 = data[1];
            let byte2 = data[2];
            let byte3 = data[3];

            if byte1 >= 1 && byte1 <= 31 && byte2 >= 1 && byte2 <= 12 && byte3 <= 99 {
                // DPT 11.001: Date
                // Byte 1: day (1-31)
                // Byte 2: month (1-12)
                // Byte 3: year (0-99)
                Some(KnxValue::Date {
                    day: byte1,
                    month: byte2,
                    year: byte3,
                })
            } else {
                // DPT 10.001: Time of day
                // Byte 1: (day<<5) | hour
                // Byte 2: minute
                // Byte 3: second
                let day = (byte1 >> 5) & 0x07;
                let hour = byte1 & 0x1F;
                Some(KnxValue::Time {
                    day,
                    hour,
                    minute: byte2,
                    second: byte3,
                })
            }
        }
        15 => {
            // DPT 16.000/16.001: String ASCII (14 bytes)
            let mut string_data = [0u8; 14];
            let mut len = 0u8;

            // Copy string data (skip APCI at data[0])
            for i in 0..14 {
                let byte = data[1 + i];
                string_data[i] = byte;
                if byte != 0 && len == i as u8 {
                    len = (i + 1) as u8;
                }
            }

            Some(KnxValue::StringAscii {
                data: string_data,
                len,
            })
        }
        9 => {
            // DPT 19.001: Date and Time (8 bytes)
            // Byte 0: Year since 1900 (0-255 = 1900-2155)
            // Byte 1: Month (1-12)
            // Byte 2: Day (1-31)
            // Byte 3: Day of week (bits 7-5) + Hour (bits 4-0)
            // Byte 4: Minute
            // Byte 5: Second
            // Byte 6: Flags 1
            // Byte 7: Flags 2
            let year = 1900 + data[1] as u16;
            let month = data[2];
            let day = data[3];
            let day_of_week = (data[4] >> 5) & 0x07;
            let hour = data[4] & 0x1F;
            let minute = data[5];
            let second = data[6];

            let flags1 = data[7];
            let flags2 = data[8];

            Some(KnxValue::DateTime {
                year,
                month,
                day,
                day_of_week,
                hour,
                minute,
                second,
                fault: (flags1 & 0x80) != 0,
                working_day: (flags1 & 0x40) != 0,
                no_wd: (flags1 & 0x20) != 0,
                no_year: (flags1 & 0x10) != 0,
                no_date: (flags1 & 0x08) != 0,
                no_dow: (flags1 & 0x04) != 0,
                no_time: (flags1 & 0x02) != 0,
                standard_summertime: (flags1 & 0x01) != 0,
                quality: (flags2 & 0x80) != 0,
            })
        }
        _ => None,
    }
}

/// Decodes DPT 9.001 (2-byte float) to f32.
///
/// # Arguments
///
/// * `value` - Encoded 16-bit value in format MEEE EMMM MMMM MMMM
///
/// # Returns
///
/// Decoded float value using formula: `(0.01 * mantissa) * 2^exponent`.
fn decode_dpt9(value: u16) -> f32 {
    let sign = (value & 0x8000) != 0;
    let exponent = ((value >> 11) & 0x0F) as i32;
    let mantissa = (value & 0x07FF) as i32;

    let mantissa = if sign { -mantissa } else { mantissa };

    (0.01 * mantissa as f32) * (1 << exponent) as f32
}

/// Formats a group address into main/middle/sub components.
///
/// # Arguments
///
/// * `addr` - Group address to format
///
/// # Returns
///
/// Tuple of `(main, middle, sub)` address components in 3-level format.
///
/// # Example
///
/// ```
/// let addr = GroupAddress::from(0x0A03); // Binary: 00001 010 00000011
/// let (main, middle, sub) = format_group_address(addr);
/// assert_eq!((main, middle, sub), (1, 2, 3)); // 1/2/3
/// ```
pub fn format_group_address(addr: GroupAddress) -> (u8, u8, u8) {
    let raw: u16 = addr.into();
    let main = ((raw >> 11) & 0x1F) as u8;
    let middle = ((raw >> 8) & 0x07) as u8;
    let sub = (raw & 0xFF) as u8;
    (main, middle, sub)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== DPT 1.xxx (Boolean) Tests ======

    #[test]
    fn test_dpt1_encode_true() {
        // DPT 1.xxx: Boolean true
        let value = KnxValue::Bool(true);
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 12);
        assert_eq!(buffer[0], 0x01); // NPDU length (1 byte)
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x81); // APCI (0x80) | 0x01 (true)
    }

    #[test]
    fn test_dpt1_encode_false() {
        // DPT 1.xxx: Boolean false
        let value = KnxValue::Bool(false);
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 12);
        assert_eq!(buffer[0], 0x01); // NPDU length (1 byte)
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x80); // APCI (0x80) | 0x00 (false)
    }

    #[test]
    fn test_dpt1_decode_true() {
        // Decode true value from 1-byte data
        let data = [0x81]; // APCI with bit 0 set
        let value = decode_value(&data);

        assert_eq!(value, Some(KnxValue::Bool(true)));
    }

    #[test]
    fn test_dpt1_decode_false() {
        // Decode false value from 1-byte data
        let data = [0x80]; // APCI with bit 0 clear
        let value = decode_value(&data);

        assert_eq!(value, Some(KnxValue::Bool(false)));
    }

    #[test]
    fn test_dpt1_roundtrip_true() {
        // Test encode/decode roundtrip for true
        let original = KnxValue::Bool(true);
        let mut buffer = [0u8; 16];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Extract data portion (APCI byte)
        let decoded = decode_value(&buffer[2..3]);

        assert_eq!(decoded, Some(original));
    }

    #[test]
    fn test_dpt1_roundtrip_false() {
        // Test encode/decode roundtrip for false
        let original = KnxValue::Bool(false);
        let mut buffer = [0u8; 16];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Extract data portion (APCI byte)
        let decoded = decode_value(&buffer[2..3]);

        assert_eq!(decoded, Some(original));
    }

    #[test]
    fn test_dpt1_type_name() {
        assert_eq!(DptType::Bool.name(), "Boolean (DPT 1.xxx)");
    }

    #[test]
    fn test_dpt1_apply_to_value() {
        // Test that apply_to_value preserves Bool values
        let value = KnxValue::Bool(true);
        let result = DptType::Bool.apply_to_value(value);
        assert_eq!(result, KnxValue::Bool(true));

        let value = KnxValue::Bool(false);
        let result = DptType::Bool.apply_to_value(value);
        assert_eq!(result, KnxValue::Bool(false));
    }

    // ====== DPT 3.xxx (Control3Bit) Tests ======

    #[test]
    fn test_dpt3_encode_decrease_break() {
        // DPT 3.xxx: decrease/up, break (step = 0)
        let value = KnxValue::Control3Bit { control: false, step: 0 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 12);
        assert_eq!(buffer[0], 0x02); // NPDU length
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x80); // APCI (write)
        assert_eq!(buffer[3], 0x00); // control=0, step=0
    }

    #[test]
    fn test_dpt3_encode_increase_step_5() {
        // DPT 3.xxx: increase/down, step = 5
        let value = KnxValue::Control3Bit { control: true, step: 5 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 12);
        assert_eq!(buffer[0], 0x02);
        assert_eq!(buffer[1], 0x00);
        assert_eq!(buffer[2], 0x80);
        assert_eq!(buffer[3], 0x0D); // 0x08 (control=1) | 0x05 (step=5) = 0x0D
    }

    #[test]
    fn test_dpt3_encode_step_clamping() {
        // Test that step > 7 gets clamped to 7
        let value = KnxValue::Control3Bit { control: false, step: 15 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 12);
        assert_eq!(buffer[3], 0x07); // Should be clamped to 7
    }

    #[test]
    fn test_dpt3_decode_from_u8() {
        // Test decoding via DPT registry
        let raw_u8 = KnxValue::U8(0x0D); // control=1, step=5

        // Apply Dimming DPT type
        let dimming_value = DptType::Dimming.apply_to_value(raw_u8);
        assert_eq!(dimming_value, KnxValue::Control3Bit { control: true, step: 5 });

        // Apply Blinds DPT type
        let blinds_value = DptType::Blinds.apply_to_value(raw_u8);
        assert_eq!(blinds_value, KnxValue::Control3Bit { control: true, step: 5 });
    }

    #[test]
    fn test_dpt3_decode_decrease_break() {
        let raw_u8 = KnxValue::U8(0x00); // control=0, step=0
        let value = DptType::Dimming.apply_to_value(raw_u8);
        assert_eq!(value, KnxValue::Control3Bit { control: false, step: 0 });
    }

    #[test]
    fn test_dpt3_all_step_values() {
        // Test all valid step values (0-7)
        for step in 0..=7 {
            for control in [false, true] {
                let expected_byte = if control {
                    0x08 | step
                } else {
                    step
                };

                let raw_u8 = KnxValue::U8(expected_byte);
                let decoded = DptType::Dimming.apply_to_value(raw_u8);
                assert_eq!(decoded, KnxValue::Control3Bit { control, step });
            }
        }
    }

    #[test]
    fn test_dpt3_roundtrip() {
        // Test encode -> decode roundtrip
        let original = KnxValue::Control3Bit { control: true, step: 3 };

        // Encode
        let mut buffer = [0u8; 16];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Decode (simulated by extracting the encoded byte)
        let encoded_byte = buffer[3];
        let raw_u8 = KnxValue::U8(encoded_byte);
        let decoded = DptType::Dimming.apply_to_value(raw_u8);

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_dpt3_type_names() {
        assert_eq!(DptType::Dimming.name(), "Dimming Control (DPT 3.007)");
        assert_eq!(DptType::Blinds.name(), "Blinds Control (DPT 3.008)");
    }

    #[test]
    fn test_dpt3_already_decoded_passthrough() {
        // Test that already-decoded Control3Bit values pass through unchanged
        let value = KnxValue::Control3Bit { control: false, step: 7 };

        let dimming_value = DptType::Dimming.apply_to_value(value);
        assert_eq!(dimming_value, value);

        let blinds_value = DptType::Blinds.apply_to_value(value);
        assert_eq!(blinds_value, value);
    }

    // ====== DPT 10.xxx (Time) Tests ======

    #[test]
    fn test_dpt10_encode_time() {
        // DPT 10.001: Monday, 14:30:45
        let value = KnxValue::Time { day: 1, hour: 14, minute: 30, second: 45 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        assert_eq!(buffer[0], 0x04); // NPDU length
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x80); // APCI (write)
        assert_eq!(buffer[3], (1 << 5) | 14); // day=1, hour=14
        assert_eq!(buffer[4], 30); // minute
        assert_eq!(buffer[5], 45); // second
    }

    #[test]
    fn test_dpt10_encode_no_day() {
        // DPT 10.001: No day, 00:00:00
        let value = KnxValue::Time { day: 0, hour: 0, minute: 0, second: 0 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        assert_eq!(buffer[3], 0x00); // day=0, hour=0
        assert_eq!(buffer[4], 0); // minute
        assert_eq!(buffer[5], 0); // second
    }

    #[test]
    fn test_dpt10_encode_sunday_midnight() {
        // DPT 10.001: Sunday (7), 23:59:59
        let value = KnxValue::Time { day: 7, hour: 23, minute: 59, second: 59 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        assert_eq!(buffer[3], (7 << 5) | 23); // day=7, hour=23
        assert_eq!(buffer[4], 59); // minute
        assert_eq!(buffer[5], 59); // second
    }

    #[test]
    fn test_dpt10_encode_clamping() {
        // Test that values > max get clamped
        let value = KnxValue::Time { day: 10, hour: 25, minute: 70, second: 80 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        // Should be clamped to day=7, hour=23, minute=59, second=59
        assert_eq!(buffer[3], (7 << 5) | 23);
        assert_eq!(buffer[4], 59);
        assert_eq!(buffer[5], 59);
    }

    #[test]
    fn test_dpt10_decode_time() {
        // Simulate received data: APCI + 3 bytes
        let data = [0x80, (3 << 5) | 15, 45, 30]; // Wednesday, 15:45:30

        let decoded = decode_value(&data);
        assert_eq!(
            decoded,
            Some(KnxValue::Time { day: 3, hour: 15, minute: 45, second: 30 })
        );
    }

    #[test]
    fn test_dpt10_roundtrip() {
        // Test encode -> decode roundtrip
        let original = KnxValue::Time { day: 5, hour: 12, minute: 30, second: 15 };

        // Encode
        let mut buffer = [0u8; 16];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Decode (simulated by extracting the encoded bytes)
        let data = [buffer[2], buffer[3], buffer[4], buffer[5]];
        let decoded = decode_value(&data);

        assert_eq!(decoded, Some(original));
    }

    #[test]
    fn test_dpt10_type_name() {
        assert_eq!(DptType::Time.name(), "Time of Day (DPT 10.001)");
    }

    #[test]
    fn test_dpt10_apply_to_value() {
        // Test that Time values pass through unchanged
        let value = KnxValue::Time { day: 2, hour: 10, minute: 30, second: 0 };
        let applied = DptType::Time.apply_to_value(value);
        assert_eq!(applied, value);
    }

    #[test]
    fn test_dpt10_all_days() {
        // Test all valid day values (0-7)
        for day in 0..=7 {
            let value = KnxValue::Time { day, hour: 12, minute: 0, second: 0 };
            let mut buffer = [0u8; 16];
            encode_value_with_apci(value, &mut buffer, 0x80);

            // Decode
            let data = [buffer[2], buffer[3], buffer[4], buffer[5]];
            let decoded = decode_value(&data);

            assert_eq!(decoded, Some(value));
        }
    }

    // ====== DPT 16.xxx (String ASCII) Tests ======

    /// Helper function to create StringAscii value from &str
    fn string_ascii_from_str(s: &str) -> KnxValue {
        let mut data = [0u8; 14];
        let bytes = s.as_bytes();
        let len = bytes.len().min(14);
        data[..len].copy_from_slice(&bytes[..len]);
        KnxValue::StringAscii {
            data,
            len: len as u8,
        }
    }

    #[test]
    fn test_dpt16_encode_short_string() {
        // DPT 16: "Hello"
        let value = string_ascii_from_str("Hello");
        let mut buffer = [0u8; 32];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 25);
        assert_eq!(buffer[0], 0x0F); // NPDU length
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x80); // APCI (write)
        assert_eq!(&buffer[3..8], b"Hello");
        assert_eq!(buffer[8], 0); // Null padding
    }

    #[test]
    fn test_dpt16_encode_full_string() {
        // DPT 16: 14 characters "Hello World!!!"
        let value = string_ascii_from_str("Hello World!!!");
        let mut buffer = [0u8; 32];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 25);
        assert_eq!(&buffer[3..17], b"Hello World!!!");
    }

    #[test]
    fn test_dpt16_encode_empty_string() {
        // DPT 16: empty string
        let value = string_ascii_from_str("");
        let mut buffer = [0u8; 32];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 25);
        // All data bytes should be null
        for i in 3..17 {
            assert_eq!(buffer[i], 0);
        }
    }

    #[test]
    fn test_dpt16_encode_with_padding() {
        // DPT 16: "Test" (should be padded with nulls)
        let value = string_ascii_from_str("Test");
        let mut buffer = [0u8; 32];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 25);
        assert_eq!(&buffer[3..7], b"Test");
        // Check null padding
        for i in 7..17 {
            assert_eq!(buffer[i], 0);
        }
    }

    #[test]
    fn test_dpt16_decode_string() {
        // Simulate received data: APCI + 14 bytes for "KNX"
        let mut data = [0u8; 15];
        data[0] = 0x80; // APCI
        data[1] = b'K';
        data[2] = b'N';
        data[3] = b'X';
        // Rest are zeros (null padding)

        let decoded = decode_value(&data);
        match decoded {
            Some(KnxValue::StringAscii { data, len }) => {
                assert_eq!(len, 3);
                assert_eq!(&data[0..3], b"KNX");
            }
            _ => panic!("Expected StringAscii, got {:?}", decoded),
        }
    }

    #[test]
    fn test_dpt16_roundtrip() {
        // Test encode -> decode roundtrip
        let original = string_ascii_from_str("Test123");

        // Encode
        let mut buffer = [0u8; 32];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Decode (extract the relevant bytes)
        let data = &buffer[2..17]; // APCI + 14 data bytes
        let decoded = decode_value(data);

        match (&original, decoded) {
            (
                KnxValue::StringAscii {
                    data: d1,
                    len: l1,
                },
                Some(KnxValue::StringAscii {
                    data: d2,
                    len: l2,
                }),
            ) => {
                assert_eq!(l1, &l2);
                assert_eq!(&d1[..], &d2[..]);
            }
            _ => panic!("Roundtrip failed"),
        }
    }

    #[test]
    fn test_dpt16_type_name() {
        assert_eq!(
            DptType::StringAscii.name(),
            "String ASCII (DPT 16.000/16.001)"
        );
    }

    #[test]
    fn test_dpt16_apply_to_value() {
        // Test that StringAscii values pass through unchanged
        let value = string_ascii_from_str("Hello");
        let applied = DptType::StringAscii.apply_to_value(value);

        match (&value, applied) {
            (
                KnxValue::StringAscii {
                    data: d1,
                    len: l1,
                },
                KnxValue::StringAscii {
                    data: d2,
                    len: l2,
                },
            ) => {
                assert_eq!(l1, &l2);
                assert_eq!(&d1[..], &d2[..]);
            }
            _ => panic!("apply_to_value failed"),
        }
    }

    #[test]
    fn test_dpt16_special_characters() {
        // Test with special ASCII characters
        let value = string_ascii_from_str("Test!@#$%");
        let mut buffer = [0u8; 32];
        encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(&buffer[3..12], b"Test!@#$%");
    }

    #[test]
    fn test_dpt16_length_clamping() {
        // Test that strings > 14 characters get clamped
        let mut data = [b'A'; 14];
        let value = KnxValue::StringAscii { data, len: 20 }; // len > 14

        let mut buffer = [0u8; 32];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 25);
        // Should encode exactly 14 'A's
        for i in 3..17 {
            assert_eq!(buffer[i], b'A');
        }
    }

    // ====== DPT 11.xxx (Date) Tests ======

    #[test]
    fn test_dpt11_encode_date() {
        // DPT 11.001: 2023-12-25 (25/12/23)
        let value = KnxValue::Date { day: 25, month: 12, year: 23 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        assert_eq!(buffer[0], 0x04); // NPDU length
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x80); // APCI (write)
        assert_eq!(buffer[3], 25); // day
        assert_eq!(buffer[4], 12); // month
        assert_eq!(buffer[5], 23); // year
    }

    #[test]
    fn test_dpt11_encode_first_day() {
        // DPT 11.001: 2000-01-01 (01/01/00)
        let value = KnxValue::Date { day: 1, month: 1, year: 0 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        assert_eq!(buffer[3], 1); // day
        assert_eq!(buffer[4], 1); // month
        assert_eq!(buffer[5], 0); // year
    }

    #[test]
    fn test_dpt11_encode_last_day() {
        // DPT 11.001: 2099-12-31 (31/12/99)
        let value = KnxValue::Date { day: 31, month: 12, year: 99 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        assert_eq!(buffer[3], 31); // day
        assert_eq!(buffer[4], 12); // month
        assert_eq!(buffer[5], 99); // year
    }

    #[test]
    fn test_dpt11_encode_clamping() {
        // Test that values out of range get clamped
        let value = KnxValue::Date { day: 0, month: 0, year: 150 };
        let mut buffer = [0u8; 16];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 14);
        // Should be clamped to day=1, month=1, year=99
        assert_eq!(buffer[3], 1);
        assert_eq!(buffer[4], 1);
        assert_eq!(buffer[5], 99);
    }

    #[test]
    fn test_dpt11_decode_date() {
        // Simulate received data: APCI + 3 bytes for 2023-06-15
        let data = [0x80, 15, 6, 23];

        let decoded = decode_value(&data);
        assert_eq!(
            decoded,
            Some(KnxValue::Date { day: 15, month: 6, year: 23 })
        );
    }

    #[test]
    fn test_dpt11_roundtrip() {
        // Test encode -> decode roundtrip
        let original = KnxValue::Date { day: 10, month: 3, year: 24 };

        // Encode
        let mut buffer = [0u8; 16];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Decode (simulated by extracting the encoded bytes)
        let data = [buffer[2], buffer[3], buffer[4], buffer[5]];
        let decoded = decode_value(&data);

        assert_eq!(decoded, Some(original));
    }

    #[test]
    fn test_dpt11_type_name() {
        assert_eq!(DptType::Date.name(), "Date (DPT 11.001)");
    }

    #[test]
    fn test_dpt11_apply_to_value() {
        // Test that Date values pass through unchanged
        let value = KnxValue::Date { day: 15, month: 8, year: 23 };
        let applied = DptType::Date.apply_to_value(value);
        assert_eq!(applied, value);
    }

    #[test]
    fn test_dpt11_all_months() {
        // Test all valid month values (1-12)
        for month in 1..=12 {
            let value = KnxValue::Date { day: 15, month, year: 23 };
            let mut buffer = [0u8; 16];
            encode_value_with_apci(value, &mut buffer, 0x80);

            // Decode
            let data = [buffer[2], buffer[3], buffer[4], buffer[5]];
            let decoded = decode_value(&data);

            assert_eq!(decoded, Some(value));
        }
    }

    #[test]
    fn test_dpt11_heuristic_distinguishes_date() {
        // Test that the heuristic correctly identifies Date
        // 2023-03-15: day=15, month=3, year=23
        let data = [0x80, 15, 3, 23];
        let decoded = decode_value(&data);

        match decoded {
            Some(KnxValue::Date { day: 15, month: 3, year: 23 }) => {}, // Expected
            _ => panic!("Expected Date, got {:?}", decoded),
        }
    }

    #[test]
    fn test_dpt11_heuristic_distinguishes_time() {
        // Test that the heuristic correctly identifies Time when data doesn't match Date pattern
        // Time with day=5, hour=18 encodes as (5<<5)|18 = 160 + 18 = 178
        let data = [0x80, 178, 30, 45];
        let decoded = decode_value(&data);

        match decoded {
            Some(KnxValue::Time { day: 5, hour: 18, minute: 30, second: 45 }) => {}, // Expected
            _ => panic!("Expected Time, got {:?}", decoded),
        }
    }

    // ====== DPT 19.xxx (Date and Time) Tests ======

    #[test]
    fn test_dpt19_encode_datetime() {
        // DPT 19.001: 2023-12-25 Monday 14:30:45, no flags
        let value = KnxValue::DateTime {
            year: 2023,
            month: 12,
            day: 25,
            day_of_week: 1, // Monday
            hour: 14,
            minute: 30,
            second: 45,
            fault: false,
            working_day: true,
            no_wd: false,
            no_year: false,
            no_date: false,
            no_dow: false,
            no_time: false,
            standard_summertime: false,
            quality: true,
        };
        let mut buffer = [0u8; 32];
        let len = encode_value_with_apci(value, &mut buffer, 0x80);

        assert_eq!(len, 19);
        assert_eq!(buffer[0], 0x09); // NPDU length
        assert_eq!(buffer[1], 0x00); // TPCI
        assert_eq!(buffer[2], 0x80); // APCI (write)
        assert_eq!(buffer[3], 123); // Year: 2023 - 1900 = 123
        assert_eq!(buffer[4], 12); // Month
        assert_eq!(buffer[5], 25); // Day
        assert_eq!(buffer[6], (1 << 5) | 14); // day_of_week=1, hour=14
        assert_eq!(buffer[7], 30); // Minute
        assert_eq!(buffer[8], 45); // Second
        assert_eq!(buffer[9], 0x40); // Flags1: working_day=1
        assert_eq!(buffer[10], 0x80); // Flags2: quality=1
    }

    #[test]
    fn test_dpt19_encode_year_bounds() {
        // Test year clamping: 1900
        let value1 = KnxValue::DateTime {
            year: 1900,
            month: 1,
            day: 1,
            day_of_week: 0,
            hour: 0,
            minute: 0,
            second: 0,
            fault: false,
            working_day: false,
            no_wd: false,
            no_year: false,
            no_date: false,
            no_dow: false,
            no_time: false,
            standard_summertime: false,
            quality: false,
        };
        let mut buffer = [0u8; 32];
        encode_value_with_apci(value1, &mut buffer, 0x80);
        assert_eq!(buffer[3], 0); // 1900 - 1900 = 0

        // Test year clamping: 2155
        let value2 = KnxValue::DateTime {
            year: 2155,
            month: 12,
            day: 31,
            day_of_week: 0,
            hour: 23,
            minute: 59,
            second: 59,
            fault: false,
            working_day: false,
            no_wd: false,
            no_year: false,
            no_date: false,
            no_dow: false,
            no_time: false,
            standard_summertime: false,
            quality: false,
        };
        encode_value_with_apci(value2, &mut buffer, 0x80);
        assert_eq!(buffer[3], 255); // 2155 - 1900 = 255
    }

    #[test]
    fn test_dpt19_encode_all_flags() {
        // Test with all flags set
        let value = KnxValue::DateTime {
            year: 2000,
            month: 6,
            day: 15,
            day_of_week: 3,
            hour: 12,
            minute: 30,
            second: 0,
            fault: true,
            working_day: true,
            no_wd: true,
            no_year: true,
            no_date: true,
            no_dow: true,
            no_time: true,
            standard_summertime: true,
            quality: true,
        };
        let mut buffer = [0u8; 32];
        encode_value_with_apci(value, &mut buffer, 0x80);

        // All flags in byte 9 should be set
        assert_eq!(buffer[9], 0xFF);
        // Quality flag in byte 10 should be set
        assert_eq!(buffer[10], 0x80);
    }

    #[test]
    fn test_dpt19_decode_datetime() {
        // Simulate received data: APCI + 8 bytes for 2023-06-15 Wednesday 10:30:45
        let mut data = [0u8; 9];
        data[0] = 0x80; // APCI
        data[1] = 123; // 2023 - 1900
        data[2] = 6; // June
        data[3] = 15; // 15th
        data[4] = (3 << 5) | 10; // Wednesday, 10:00
        data[5] = 30; // 30 minutes
        data[6] = 45; // 45 seconds
        data[7] = 0x40; // working_day flag
        data[8] = 0x80; // quality flag

        let decoded = decode_value(&data);
        match decoded {
            Some(KnxValue::DateTime {
                year,
                month,
                day,
                day_of_week,
                hour,
                minute,
                second,
                fault,
                working_day,
                no_wd,
                no_year,
                no_date,
                no_dow,
                no_time,
                standard_summertime,
                quality,
            }) => {
                assert_eq!(year, 2023);
                assert_eq!(month, 6);
                assert_eq!(day, 15);
                assert_eq!(day_of_week, 3);
                assert_eq!(hour, 10);
                assert_eq!(minute, 30);
                assert_eq!(second, 45);
                assert_eq!(fault, false);
                assert_eq!(working_day, true);
                assert_eq!(no_wd, false);
                assert_eq!(no_year, false);
                assert_eq!(no_date, false);
                assert_eq!(no_dow, false);
                assert_eq!(no_time, false);
                assert_eq!(standard_summertime, false);
                assert_eq!(quality, true);
            }
            _ => panic!("Expected DateTime, got {:?}", decoded),
        }
    }

    #[test]
    fn test_dpt19_roundtrip() {
        // Test encode -> decode roundtrip
        let original = KnxValue::DateTime {
            year: 2024,
            month: 3,
            day: 10,
            day_of_week: 7, // Sunday
            hour: 18,
            minute: 45,
            second: 30,
            fault: false,
            working_day: false,
            no_wd: false,
            no_year: false,
            no_date: false,
            no_dow: false,
            no_time: false,
            standard_summertime: true,
            quality: false,
        };

        // Encode
        let mut buffer = [0u8; 32];
        encode_value_with_apci(original, &mut buffer, 0x80);

        // Decode (extract the relevant bytes)
        let data = &buffer[2..11]; // APCI + 8 data bytes
        let decoded = decode_value(data);

        // Compare
        match (&original, decoded) {
            (KnxValue::DateTime { .. }, Some(KnxValue::DateTime { .. })) => {
                // Both are DateTime, values should match
                assert_eq!(Some(original), decoded);
            }
            _ => panic!("Roundtrip failed"),
        }
    }

    #[test]
    fn test_dpt19_type_name() {
        assert_eq!(DptType::DateTime.name(), "Date and Time (DPT 19.001)");
    }

    // ====== Integration Tests ======

    #[test]
    fn test_control3bit_equality() {
        let a = KnxValue::Control3Bit { control: true, step: 5 };
        let b = KnxValue::Control3Bit { control: true, step: 5 };
        let c = KnxValue::Control3Bit { control: false, step: 5 };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_control3bit_debug_format() {
        let value = KnxValue::Control3Bit { control: true, step: 3 };
        let debug_str = format!("{:?}", value);
        assert!(debug_str.contains("Control3Bit"));
        assert!(debug_str.contains("control"));
        assert!(debug_str.contains("step"));
    }

    #[test]
    fn test_time_equality() {
        let a = KnxValue::Time { day: 1, hour: 12, minute: 30, second: 45 };
        let b = KnxValue::Time { day: 1, hour: 12, minute: 30, second: 45 };
        let c = KnxValue::Time { day: 2, hour: 12, minute: 30, second: 45 };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_time_debug_format() {
        let value = KnxValue::Time { day: 3, hour: 15, minute: 45, second: 30 };
        let debug_str = format!("{:?}", value);
        assert!(debug_str.contains("Time"));
        assert!(debug_str.contains("day"));
        assert!(debug_str.contains("hour"));
        assert!(debug_str.contains("minute"));
        assert!(debug_str.contains("second"));
    }

    #[test]
    fn test_date_equality() {
        let a = KnxValue::Date { day: 15, month: 6, year: 23 };
        let b = KnxValue::Date { day: 15, month: 6, year: 23 };
        let c = KnxValue::Date { day: 16, month: 6, year: 23 };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_date_debug_format() {
        let value = KnxValue::Date { day: 25, month: 12, year: 23 };
        let debug_str = format!("{:?}", value);
        assert!(debug_str.contains("Date"));
        assert!(debug_str.contains("day"));
        assert!(debug_str.contains("month"));
        assert!(debug_str.contains("year"));
    }
}
