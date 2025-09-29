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

/// High-level KNX event
#[derive(Debug)]
pub enum KnxEvent {
    /// Light switched on/off
    LightSwitch { address: GroupAddress, on: bool },
    /// Group value read request
    ValueRead { address: GroupAddress },
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

    /// Send a boolean value (DPT 1) to a group address
    pub async fn send_bool(&mut self, address: GroupAddress, value: bool) -> Result<(), ()> {
        let cemi = build_group_write_bool(address, value);
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
                                // Decode boolean value (DPT 1)
                                if ldata.data.len() == 1 {
                                    let value = (ldata.data[0] & 0x01) != 0;
                                    return Ok(Some(KnxEvent::LightSwitch {
                                        address: dest,
                                        on: value,
                                    }));
                                } else {
                                    return Ok(Some(KnxEvent::Unknown {
                                        address: dest,
                                        data_len: ldata.data.len(),
                                    }));
                                }
                            } else if ldata.is_group_read() {
                                return Ok(Some(KnxEvent::ValueRead { address: dest }));
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

/// Helper: Build cEMI L_Data.req frame for GroupValue_Write with boolean
fn build_group_write_bool(group_addr: GroupAddress, value: bool) -> [u8; 11] {
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

    // TPCI/APCI
    frame[9] = 0x00;

    // APCI + data
    let apci_data = if value { 0x81 } else { 0x80 };
    frame[10] = apci_data;

    frame
}

/// Helper: Format group address as main/middle/sub
pub fn format_group_address(addr: GroupAddress) -> (u8, u8, u8) {
    let raw: u16 = addr.into();
    let main = ((raw >> 11) & 0x1F) as u8;
    let middle = ((raw >> 8) & 0x07) as u8;
    let sub = (raw & 0xFF) as u8;
    (main, middle, sub)
}
