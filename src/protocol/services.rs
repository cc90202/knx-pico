//! KNXnet/IP service request and response builders.
//!
//! This module provides zero-copy builders for constructing KNXnet/IP service
//! frames used in tunneling communication. All builders work with provided
//! buffers to avoid heap allocations, making them suitable for embedded systems.
//!
//! ## Supported Services
//!
//! - **CONNECT** - Establish tunnel connection with gateway
//! - **CONNECTIONSTATE** - Heartbeat/keep-alive checks
//! - **DISCONNECT** - Clean connection shutdown
//! - **TUNNELING** - Send/receive KNX telegrams through tunnel
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::protocol::services::ConnectRequest;
//! use knx_pico::protocol::frame::Hpai;
//!
//! // Build a connection request
//! let control = Hpai::new([192, 168, 1, 100], 3671);
//! let data = Hpai::new([192, 168, 1, 100], 3671);
//! let request = ConnectRequest::new(control, data);
//!
//! // Encode to buffer
//! let mut buffer = [0u8; 32];
//! let len = request.build(&mut buffer)?;
//! // Send buffer[..len] to gateway
//! ```
//!
//! ## Protocol Flow
//!
//! ```text
//! Client                          Gateway
//!   |                                |
//!   |------- CONNECT_REQUEST ------->|
//!   |<------ CONNECT_RESPONSE -------|
//!   |                                |
//!   |------ TUNNELING_REQUEST ------>|
//!   |<------ TUNNELING_ACK ----------|
//!   |                                |
//!   |--- CONNECTIONSTATE_REQUEST --->|  (every 60s)
//!   |<-- CONNECTIONSTATE_RESPONSE ---|
//!   |                                |
//!   |------ DISCONNECT_REQUEST ----->|
//!   |<----- DISCONNECT_RESPONSE -----|
//! ```

use crate::error::{KnxError, Result};
use crate::protocol::constants::{SERVICE_CONNECT_REQUEST, SERVICE_CONNECTIONSTATE_REQUEST, SERVICE_DISCONNECT_REQUEST, SERVICE_TUNNELING_REQUEST, SERVICE_TUNNELING_ACK};
use crate::protocol::frame::Hpai;

/// Connection Request Information (CRI) for tunneling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionRequestInfo {
    /// Connection type (`TUNNEL_CONNECTION` = 0x04)
    pub connection_type: u8,
    /// KNX layer (`TUNNEL_LINKLAYER` = 0x02)
    pub knx_layer: u8,
}

impl ConnectionRequestInfo {
    /// Create a new CRI for tunnel link layer connection
    pub const fn tunnel_link_layer() -> Self {
        Self {
            connection_type: 0x04, // TUNNEL_CONNECTION
            knx_layer: 0x02,       // TUNNEL_LINKLAYER
        }
    }

    /// Encode CRI to bytes
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 4 {
            return Err(KnxError::buffer_too_small());
        }

        buf[0] = 4; // Structure length
        buf[1] = self.connection_type;
        buf[2] = self.knx_layer;
        buf[3] = 0x00; // Reserved

        Ok(4)
    }

    /// Decode CRI from bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(KnxError::buffer_too_small());
        }

        let length = data[0];
        if length != 4 {
            return Err(KnxError::invalid_frame());
        }

        Ok(Self {
            connection_type: data[1],
            knx_layer: data[2],
        })
    }
}

/// `CONNECT_REQUEST` service (0x0205)
#[derive(Debug, Clone, Copy)]
pub struct ConnectRequest {
    /// Control endpoint (for connection management)
    pub control_endpoint: Hpai,
    /// Data endpoint (for tunneling data)
    pub data_endpoint: Hpai,
    /// Connection request information
    pub cri: ConnectionRequestInfo,
}

impl ConnectRequest {
    /// Create a new `CONNECT_REQUEST`
    pub const fn new(control_endpoint: Hpai, data_endpoint: Hpai) -> Self {
        Self {
            control_endpoint,
            data_endpoint,
            cri: ConnectionRequestInfo::tunnel_link_layer(),
        }
    }

    /// Build the complete frame
    ///
    /// Returns the number of bytes written to the buffer
    pub fn build(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 26 {
            return Err(KnxError::buffer_too_small());
        }

        let mut offset = 0;

        // Header (6 bytes)
        buf[0] = 0x06; // Header length
        buf[1] = 0x10; // Protocol version 1.0
        buf[2..4].copy_from_slice(&SERVICE_CONNECT_REQUEST.to_be_bytes());
        // Total length will be filled at the end
        offset += 6;

        // Control endpoint (8 bytes)
        offset += self.control_endpoint.encode(&mut buf[offset..])?;

        // Data endpoint (8 bytes)
        offset += self.data_endpoint.encode(&mut buf[offset..])?;

        // CRI (4 bytes)
        offset += self.cri.encode(&mut buf[offset..])?;

        // Fill total length
        buf[4..6].copy_from_slice(&(offset as u16).to_be_bytes());

        Ok(offset)
    }
}

/// `CONNECT_RESPONSE` service (0x0206)
#[derive(Debug, Clone, Copy)]
pub struct ConnectResponse {
    /// Communication channel ID
    pub channel_id: u8,
    /// Status code (0 = OK)
    pub status: u8,
    /// Data endpoint assigned by server
    pub data_endpoint: Hpai,
    /// Connection response data
    pub crd: [u8; 4],
}

impl ConnectResponse {
    /// Parse from frame body
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 14 {
            return Err(KnxError::buffer_too_small());
        }

        let channel_id = data[0];
        let status = data[1];
        let data_endpoint = Hpai::parse(&data[2..10])?;

        let mut crd = [0u8; 4];
        crd.copy_from_slice(&data[10..14]);

        Ok(Self {
            channel_id,
            status,
            data_endpoint,
            crd,
        })
    }

    /// Check if connection was successful
    pub const fn is_ok(&self) -> bool {
        self.status == 0
    }
}

/// `CONNECTIONSTATE_REQUEST` service (0x0207)
#[derive(Debug, Clone, Copy)]
pub struct ConnectionStateRequest {
    /// Communication channel ID
    pub channel_id: u8,
    /// Control endpoint
    pub control_endpoint: Hpai,
}

impl ConnectionStateRequest {
    /// Create a new `CONNECTIONSTATE_REQUEST`
    pub const fn new(channel_id: u8, control_endpoint: Hpai) -> Self {
        Self {
            channel_id,
            control_endpoint,
        }
    }

    /// Build the complete frame
    pub fn build(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 16 {
            return Err(KnxError::buffer_too_small());
        }

        let mut offset = 0;

        // Header
        buf[0] = 0x06;
        buf[1] = 0x10;
        buf[2..4].copy_from_slice(&SERVICE_CONNECTIONSTATE_REQUEST.to_be_bytes());
        offset += 6;

        // Channel ID + reserved
        buf[offset] = self.channel_id;
        buf[offset + 1] = 0x00;
        offset += 2;

        // Control endpoint
        offset += self.control_endpoint.encode(&mut buf[offset..])?;

        // Fill total length
        buf[4..6].copy_from_slice(&(offset as u16).to_be_bytes());

        Ok(offset)
    }
}

/// `CONNECTIONSTATE_RESPONSE` service (0x0208)
#[derive(Debug, Clone, Copy)]
pub struct ConnectionStateResponse {
    /// Communication channel ID
    pub channel_id: u8,
    /// Status code (0 = OK)
    pub status: u8,
}

impl ConnectionStateResponse {
    /// Parse from frame body
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(KnxError::buffer_too_small());
        }

        Ok(Self {
            channel_id: data[0],
            status: data[1],
        })
    }

    /// Check if connection is still alive
    pub const fn is_ok(&self) -> bool {
        self.status == 0
    }
}

/// `DISCONNECT_REQUEST` service (0x0209)
#[derive(Debug, Clone, Copy)]
pub struct DisconnectRequest {
    /// Communication channel ID
    pub channel_id: u8,
    /// Control endpoint
    pub control_endpoint: Hpai,
}

impl DisconnectRequest {
    /// Create a new `DISCONNECT_REQUEST`
    pub const fn new(channel_id: u8, control_endpoint: Hpai) -> Self {
        Self {
            channel_id,
            control_endpoint,
        }
    }

    /// Build the complete frame
    pub fn build(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 16 {
            return Err(KnxError::buffer_too_small());
        }

        let mut offset = 0;

        // Header
        buf[0] = 0x06;
        buf[1] = 0x10;
        buf[2..4].copy_from_slice(&SERVICE_DISCONNECT_REQUEST.to_be_bytes());
        offset += 6;

        // Channel ID + reserved
        buf[offset] = self.channel_id;
        buf[offset + 1] = 0x00;
        offset += 2;

        // Control endpoint
        offset += self.control_endpoint.encode(&mut buf[offset..])?;

        // Fill total length
        buf[4..6].copy_from_slice(&(offset as u16).to_be_bytes());

        Ok(offset)
    }
}

/// `DISCONNECT_RESPONSE` service (0x020A)
#[derive(Debug, Clone, Copy)]
pub struct DisconnectResponse {
    /// Communication channel ID
    pub channel_id: u8,
    /// Status code (0 = OK)
    pub status: u8,
}

impl DisconnectResponse {
    /// Parse from frame body
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(KnxError::buffer_too_small());
        }

        Ok(Self {
            channel_id: data[0],
            status: data[1],
        })
    }

    /// Check if disconnect was acknowledged
    pub const fn is_ok(&self) -> bool {
        self.status == 0
    }
}

/// Connection header for tunneling requests
#[derive(Debug, Clone, Copy)]
pub struct ConnectionHeader {
    /// Communication channel ID
    pub channel_id: u8,
    /// Sequence counter
    pub sequence_counter: u8,
}

impl ConnectionHeader {
    /// Create a new connection header
    pub const fn new(channel_id: u8, sequence_counter: u8) -> Self {
        Self {
            channel_id,
            sequence_counter,
        }
    }

    /// Encode to bytes
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 4 {
            return Err(KnxError::buffer_too_small());
        }

        buf[0] = 4; // Structure length
        buf[1] = self.channel_id;
        buf[2] = self.sequence_counter;
        buf[3] = 0x00; // Reserved

        Ok(4)
    }

    /// Decode from bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(KnxError::buffer_too_small());
        }

        Ok(Self {
            channel_id: data[1],
            sequence_counter: data[2],
        })
    }
}

/// `TUNNELING_REQUEST` service (0x0420)
#[derive(Debug)]
pub struct TunnelingRequest<'a> {
    /// Connection header
    pub connection_header: ConnectionHeader,
    /// cEMI frame data
    pub cemi_data: &'a [u8],
}

impl<'a> TunnelingRequest<'a> {
    /// Create a new `TUNNELING_REQUEST`
    pub const fn new(connection_header: ConnectionHeader, cemi_data: &'a [u8]) -> Self {
        Self {
            connection_header,
            cemi_data,
        }
    }

    /// Build the complete frame
    pub fn build(&self, buf: &mut [u8]) -> Result<usize> {
        let total_len = 6 + 4 + self.cemi_data.len();
        if buf.len() < total_len {
            return Err(KnxError::buffer_too_small());
        }

        let mut offset = 0;

        // Header
        buf[0] = 0x06;
        buf[1] = 0x10;
        buf[2..4].copy_from_slice(&SERVICE_TUNNELING_REQUEST.to_be_bytes());
        buf[4..6].copy_from_slice(&(total_len as u16).to_be_bytes());
        offset += 6;

        // Connection header
        offset += self.connection_header.encode(&mut buf[offset..])?;

        // cEMI data
        buf[offset..offset + self.cemi_data.len()].copy_from_slice(self.cemi_data);
        offset += self.cemi_data.len();

        Ok(offset)
    }

    /// Parse from frame body
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(KnxError::buffer_too_small());
        }

        let connection_header = ConnectionHeader::decode(&data[0..4])?;
        let cemi_data = &data[4..];

        Ok(Self {
            connection_header,
            cemi_data,
        })
    }
}

/// `TUNNELING_ACK` service (0x0421)
#[derive(Debug, Clone, Copy)]
pub struct TunnelingAck {
    /// Connection header
    pub connection_header: ConnectionHeader,
    /// Status code (0 = OK)
    pub status: u8,
}

impl TunnelingAck {
    /// Create a new `TUNNELING_ACK`
    pub const fn new(connection_header: ConnectionHeader, status: u8) -> Self {
        Self {
            connection_header,
            status,
        }
    }

    /// Build the complete frame
    pub fn build(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 11 {
            return Err(KnxError::buffer_too_small());
        }

        let mut offset = 0;

        // Header
        buf[0] = 0x06;
        buf[1] = 0x10;
        buf[2..4].copy_from_slice(&SERVICE_TUNNELING_ACK.to_be_bytes());
        buf[4..6].copy_from_slice(&11u16.to_be_bytes());
        offset += 6;

        // Connection header
        offset += self.connection_header.encode(&mut buf[offset..])?;

        // Status
        buf[offset] = self.status;
        offset += 1;

        Ok(offset)
    }

    /// Parse from frame body
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(KnxError::buffer_too_small());
        }

        let connection_header = ConnectionHeader::decode(&data[0..4])?;
        let status = data[4];

        Ok(Self {
            connection_header,
            status,
        })
    }

    /// Check if request was acknowledged successfully
    pub const fn is_ok(&self) -> bool {
        self.status == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hpai_encode_decode() {
        let hpai = Hpai::new([192, 168, 1, 10], 3671);
        let mut buf = [0u8; 8];
        let len = hpai.encode(&mut buf).unwrap();
        assert_eq!(len, 8);

        let decoded = Hpai::parse(&buf).unwrap();
        assert_eq!(decoded, hpai);
    }

    #[test]
    fn test_connect_request_build() {
        let control = Hpai::new([192, 168, 1, 100], 3671);
        let data = Hpai::new([192, 168, 1, 100], 3671);
        let request = ConnectRequest::new(control, data);

        let mut buf = [0u8; 32];
        let len = request.build(&mut buf).unwrap();

        assert_eq!(len, 26);
        assert_eq!(&buf[0..2], &[0x06, 0x10]); // Header
        assert_eq!(u16::from_be_bytes([buf[2], buf[3]]), SERVICE_CONNECT_REQUEST);
    }

    #[test]
    fn test_connection_header() {
        let header = ConnectionHeader::new(5, 10);
        let mut buf = [0u8; 4];
        let len = header.encode(&mut buf).unwrap();

        assert_eq!(len, 4);

        let decoded = ConnectionHeader::decode(&buf).unwrap();
        assert_eq!(decoded.channel_id, 5);
        assert_eq!(decoded.sequence_counter, 10);
    }

    #[test]
    fn test_tunneling_ack() {
        let header = ConnectionHeader::new(3, 15);
        let ack = TunnelingAck::new(header, 0);

        let mut buf = [0u8; 16];
        let len = ack.build(&mut buf).unwrap();

        assert_eq!(len, 11);
        assert!(ack.is_ok());
    }
}
