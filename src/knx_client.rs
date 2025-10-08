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
}
