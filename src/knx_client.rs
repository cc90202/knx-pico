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
//!     .build(&stack, &mut buffers)?;
//!
//! // Connect and use
//! client.connect().await?;
//! client.write(GroupAddress::from(0x0A03), KnxValue::Bool(true)).await?;
//! ```

use embassy_net::udp::PacketMetadata;
use knx_rs::addressing::{GroupAddress, IndividualAddress};
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

/// KNX value types representing different Datapoint Types (DPT).
///
/// This enum provides type-safe representations of common KNX datapoint types.
/// Each variant corresponds to a specific DPT specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KnxValue {
    /// Boolean value (DPT 1.xxx) - switch, enable/disable
    Bool(bool),
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
    ) -> Result<KnxClient<'a>, ()> {
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
    ) -> Result<KnxClient<'a>, ()> {
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

    /// Establishes connection to the KNX gateway.
    ///
    /// Must be called before any other operations.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` if connection fails.
    pub async fn connect(&mut self) -> Result<(), ()> {
        self.tunnel.connect().await.map_err(|_| ())
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
    /// Returns `Err(())` if the write operation fails.
    pub async fn write(&mut self, address: GroupAddress, value: KnxValue) -> Result<(), ()> {
        let mut buffer = [0u8; 16];
        let len = build_group_write(address, value, self.device_address, &mut buffer);
        self.tunnel.send_cemi(&buffer[..len]).await.map_err(|_| ())
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
    /// Returns `Err(())` if the read request fails.
    pub async fn read(&mut self, address: GroupAddress) -> Result<(), ()> {
        let cemi = build_group_read(address, self.device_address);
        self.tunnel.send_cemi(&cemi).await.map_err(|_| ())
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
    /// Returns `Err(())` if the response operation fails.
    pub async fn respond(&mut self, address: GroupAddress, value: KnxValue) -> Result<(), ()> {
        let mut buffer = [0u8; 16];
        let len = build_group_response(address, value, self.device_address, &mut buffer);
        self.tunnel.send_cemi(&buffer[..len]).await.map_err(|_| ())
    }

    /// Sends a raw cEMI frame (for advanced usage).
    ///
    /// # Arguments
    ///
    /// * `cemi` - Raw cEMI frame bytes
    ///
    /// # Errors
    ///
    /// Returns `Err(())` if sending fails.
    pub async fn send_raw_cemi(&mut self, cemi: &[u8]) -> Result<(), ()> {
        self.tunnel.send_cemi(cemi).await.map_err(|_| ())
    }

    /// Waits for and parses the next KNX bus event.
    ///
    /// Returns `Ok(None)` on timeout (no data available).
    ///
    /// # Returns
    ///
    /// * `Ok(Some(event))` - Parsed KNX event
    /// * `Ok(None)` - Timeout, no data available
    /// * `Err(())` - Receive error
    pub async fn receive_event(&mut self) -> Result<Option<KnxEvent>, ()> {
        match self.tunnel.receive().await {
            Ok(Some(cemi_data)) => {
                if let Ok(cemi) = knx_rs::protocol::cemi::CEMIFrame::parse(cemi_data) {
                    if let Ok(ldata) = cemi.as_ldata() {
                        if let Some(dest) = ldata.destination_group() {
                            if !ldata.data.is_empty() {
                                let apci = ldata.data[0] & 0xC0;

                                if apci == 0x80 {
                                    if let Some(value) = decode_value(ldata.data) {
                                        return Ok(Some(KnxEvent::GroupWrite {
                                            address: dest,
                                            value,
                                        }));
                                    } else {
                                        return Ok(Some(KnxEvent::Unknown {
                                            address: dest,
                                            data_len: ldata.data.len(),
                                        }));
                                    }
                                } else if apci == 0x40 {
                                    if let Some(value) = decode_value(ldata.data) {
                                        return Ok(Some(KnxEvent::GroupResponse {
                                            address: dest,
                                            value,
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
            Err(_) => Err(()),
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
/// - 1-byte data: Returns [`KnxValue::U8`] (could also be `Percent`)
/// - 2-byte unsigned: Returns [`KnxValue::U16`]
/// - 2-byte float: Returns [`KnxValue::Float2`] (could be `Temperature`, `Lux`, etc.)
///
/// Application should interpret based on group address context.
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
