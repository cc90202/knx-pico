//! High-level KNX client wrapper
//!
//! This module provides a clean API on top of AsyncTunnelClient
//! for common KNX operations.

use embassy_net::udp::PacketMetadata;
use knx_rs::addressing::{GroupAddress, IndividualAddress};
use knx_rs::protocol::async_tunnel::AsyncTunnelClient;
use knx_rs::protocol::cemi::{ControlField1, ControlField2};
use knx_rs::protocol::constants::CEMIMessageCode;

const DEVICE_ADDRESS_RAW: u16 = 0x1101; // 1.1.1

/// KNX value types (Datapoint Types)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KnxValue {
    /// Boolean value (DPT 1.xxx)
    Bool(bool),
    /// Percentage (DPT 5.001) - 0-100%
    Percent(u8),
    /// Temperature in Celsius (DPT 9.001) - -273.0 to +670760.0
    Temperature(f32),
}

/// High-level KNX event
#[derive(Debug)]
pub enum KnxEvent {
    /// Group value write
    GroupWrite { address: GroupAddress, value: KnxValue },
    /// Group value read request
    GroupRead { address: GroupAddress },
    /// Unknown/unparsed event
    Unknown { address: GroupAddress, data_len: usize },
}

/// High-level KNX client wrapper
pub struct KnxClient<'a> {
    tunnel: AsyncTunnelClient<'a>,
}

impl<'a> KnxClient<'a> {
    /// Create a new KNX client
    pub fn new(
        stack: &'a embassy_net::Stack<'static>,
        rx_meta: &'a mut [PacketMetadata],
        tx_meta: &'a mut [PacketMetadata],
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
        gateway_ip: [u8; 4],
        gateway_port: u16,
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

        Self { tunnel }
    }

    /// Connect to KNX gateway
    pub async fn connect(&mut self) -> Result<(), ()> {
        self.tunnel.connect().await.map_err(|_| ())
    }

    /// Write a value to a group address (GroupValue_Write)
    pub async fn write(&mut self, address: GroupAddress, value: KnxValue) -> Result<(), ()> {
        let mut buffer = [0u8; 16]; // Max frame size
        let len = build_group_write(address, value, &mut buffer);
        self.tunnel.send_cemi(&buffer[..len]).await.map_err(|_| ())
    }

    /// Request to read a value from a group address (GroupValue_Read)
    pub async fn read(&mut self, address: GroupAddress) -> Result<(), ()> {
        let cemi = build_group_read(address);
        self.tunnel.send_cemi(&cemi).await.map_err(|_| ())
    }

    /// Send raw cEMI frame (for advanced usage)
    pub async fn send_raw_cemi(&mut self, cemi: &[u8]) -> Result<(), ()> {
        self.tunnel.send_cemi(cemi).await.map_err(|_| ())
    }

    /// Wait for and parse next KNX bus event
    pub async fn receive_event(&mut self) -> Result<Option<KnxEvent>, ()> {
        match self.tunnel.receive().await {
            Ok(Some(cemi_data)) => {
                // Parse cEMI frame
                if let Ok(cemi) = knx_rs::protocol::cemi::CEMIFrame::parse(cemi_data) {
                    if let Ok(ldata) = cemi.as_ldata() {
                        // Extract destination group address
                        if let Some(dest) = ldata.destination_group() {
                            // Check message type
                            if ldata.is_group_write() {
                                // Try to decode value
                                if let Some(value) = decode_value(&ldata.data) {
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
                            } else if ldata.is_group_read() {
                                return Ok(Some(KnxEvent::GroupRead { address: dest }));
                            }
                        }
                    }
                }
                // Parsing failed or unsupported frame type
                Ok(None)
            }
            Ok(None) => Ok(None), // Timeout, no data
            Err(_) => Err(()),     // Error
        }
    }
}

/// Helper: Build cEMI L_Data.req frame for GroupValue_Write
/// Returns the frame length
fn build_group_write(group_addr: GroupAddress, value: KnxValue, buffer: &mut [u8]) -> usize {
    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    // Message code: L_Data.req
    buffer[0] = CEMIMessageCode::LDataReq.to_u8();

    // Additional info length: 0
    buffer[1] = 0x00;

    // Control fields
    buffer[2] = ControlField1::default().raw();
    buffer[3] = ControlField2::default().raw();

    // Source address
    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    buffer[4] = source_bytes[0];
    buffer[5] = source_bytes[1];

    // Destination address
    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    buffer[6] = dest_bytes[0];
    buffer[7] = dest_bytes[1];

    // TPCI/APCI (GroupValue_Write = 0x00)
    buffer[9] = 0x00;

    // Encode value and return frame length
    encode_value(value, &mut buffer[8..])
}

/// Helper: Build cEMI L_Data.req frame for GroupValue_Read
fn build_group_read(group_addr: GroupAddress) -> [u8; 11] {
    let mut frame = [0u8; 11];
    let device_addr = IndividualAddress::from(DEVICE_ADDRESS_RAW);

    // Message code: L_Data.req
    frame[0] = CEMIMessageCode::LDataReq.to_u8();

    // Additional info length: 0
    frame[1] = 0x00;

    // Control fields
    frame[2] = ControlField1::default().raw();
    frame[3] = ControlField2::default().raw();

    // Source address
    let source_raw: u16 = device_addr.into();
    let source_bytes = source_raw.to_be_bytes();
    frame[4] = source_bytes[0];
    frame[5] = source_bytes[1];

    // Destination address
    let dest_raw: u16 = group_addr.into();
    let dest_bytes = dest_raw.to_be_bytes();
    frame[6] = dest_bytes[0];
    frame[7] = dest_bytes[1];

    // NPDU length
    frame[8] = 0x01;

    // TPCI/APCI (GroupValue_Read = 0x00)
    frame[9] = 0x00;

    // APCI only (no data for read request)
    frame[10] = 0x00;

    frame
}

/// Encode KnxValue to NPDU length + TPCI/APCI + data
/// Buffer should start at byte 8 (NPDU length position)
/// Returns total frame length
fn encode_value(value: KnxValue, buffer: &mut [u8]) -> usize {
    match value {
        KnxValue::Bool(b) => {
            // DPT 1: 6-bit data encoded in APCI byte
            // NPDU length = 1 (only TPCI/APCI + data byte)
            buffer[0] = 0x01; // NPDU length
            buffer[1] = 0x00; // TPCI/APCI
            buffer[2] = if b { 0x81 } else { 0x80 }; // APCI + 6-bit data
            11 // Total frame length
        }
        KnxValue::Percent(p) => {
            // DPT 5.001: 1 byte unsigned (0-100% mapped to 0-255)
            let value = ((p.min(100) as u16 * 255) / 100) as u8;
            buffer[0] = 0x02; // NPDU length
            buffer[1] = 0x00; // TPCI/APCI
            buffer[2] = 0x80; // APCI (GroupValue_Write)
            buffer[3] = value; // 1 byte data
            12 // Total frame length
        }
        KnxValue::Temperature(t) => {
            // DPT 9.001: 2-byte float (KNX format)
            let encoded = encode_dpt9(t);
            buffer[0] = 0x03; // NPDU length
            buffer[1] = 0x00; // TPCI/APCI
            buffer[2] = 0x80; // APCI (GroupValue_Write)
            buffer[3] = (encoded >> 8) as u8; // High byte
            buffer[4] = (encoded & 0xFF) as u8; // Low byte
            13 // Total frame length
        }
    }
}

/// Encode f32 to DPT 9.001 (2-byte float)
fn encode_dpt9(value: f32) -> u16 {
    // DPT 9: (0.01 * m) * 2^e
    // Range: -671088.64 to +670760.96

    let value = value.clamp(-671088.64, 670760.96);

    // Find exponent and mantissa
    let mut exponent = 0i32;
    let mut mantissa = (value * 100.0) as i32;

    // Normalize mantissa to 11-bit signed (-2048 to +2047)
    while mantissa < -2048 || mantissa > 2047 {
        mantissa >>= 1;
        exponent += 1;
    }

    // Clamp exponent to 4-bit (0-15)
    exponent = exponent.clamp(0, 15);

    // Build 16-bit value: MEEE EMMM MMMM MMMM
    // M = mantissa sign bit, E = exponent, M = mantissa
    let sign = if mantissa < 0 { 1u16 << 15 } else { 0 };
    let exp_bits = ((exponent as u16) & 0x0F) << 11;
    let mantissa_bits = (mantissa.abs() as u16) & 0x07FF;

    sign | exp_bits | mantissa_bits
}

/// Decode APCI+data bytes to KnxValue
fn decode_value(data: &[u8]) -> Option<KnxValue> {
    match data.len() {
        1 => {
            // DPT 1: Boolean (6-bit data in APCI byte)
            let value = (data[0] & 0x01) != 0;
            Some(KnxValue::Bool(value))
        }
        2 => {
            // DPT 5.001: 1 byte unsigned (0-255 mapped to 0-100%)
            let raw = data[1];
            let percent = ((raw as u16 * 100) / 255) as u8;
            Some(KnxValue::Percent(percent))
        }
        3 => {
            // DPT 9.001: 2-byte float
            let raw = ((data[1] as u16) << 8) | (data[2] as u16);
            let temp = decode_dpt9(raw);
            Some(KnxValue::Temperature(temp))
        }
        _ => None,
    }
}

/// Decode DPT 9.001 (2-byte float) to f32
fn decode_dpt9(value: u16) -> f32 {
    // DPT 9: (0.01 * m) * 2^e
    // Format: MEEE EMMM MMMM MMMM

    let sign = (value & 0x8000) != 0;
    let exponent = ((value >> 11) & 0x0F) as i32;
    let mantissa = (value & 0x07FF) as i32;

    // Apply sign to mantissa
    let mantissa = if sign { -mantissa } else { mantissa };

    // Calculate: (0.01 * mantissa) * 2^exponent
    (0.01 * mantissa as f32) * (1 << exponent) as f32
}

/// Helper: Format group address as main/middle/sub
pub fn format_group_address(addr: GroupAddress) -> (u8, u8, u8) {
    let raw: u16 = addr.into();
    let main = ((raw >> 11) & 0x1F) as u8;
    let middle = ((raw >> 8) & 0x07) as u8;
    let sub = (raw & 0xFF) as u8;
    (main, middle, sub)
}
