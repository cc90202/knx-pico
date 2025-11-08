//! Common External Message Interface (cEMI) implementation.
//!
//! cEMI provides the standardized interface for KNX communication, encapsulating
//! KNX telegrams within KNXnet/IP frames. This module handles parsing and building
//! of cEMI frames, including `L_Data` frames for group communication.
//!
//! ## Overview
//!
//! The cEMI protocol layer sits between KNXnet/IP (transport) and the KNX
//! application layer. It defines how to encode KNX telegrams for transmission
//! over IP networks.
//!
//! ## Frame Structure
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ Message Code (1 byte)                    │
//! ├──────────────────────────────────────────┤
//! │ Additional Info Length (1 byte)          │
//! ├──────────────────────────────────────────┤
//! │ Additional Info (variable)               │
//! ├──────────────────────────────────────────┤
//! │ Service Information (L_Data)             │
//! │  ├─ Control Field 1 (1 byte)             │
//! │  ├─ Control Field 2 (1 byte)             │
//! │  ├─ Source Address (2 bytes)             │
//! │  ├─ Destination Address (2 bytes)        │
//! │  ├─ NPDU Length (1 byte)                 │
//! │  ├─ TPCI/APCI (1-2 bytes)                │
//! │  └─ Data (variable)                      │
//! └──────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use knx_pico::protocol::cemi::{CEMIFrame, LDataFrame};
//!
//! // Parse a complete cEMI frame
//! let cemi = CEMIFrame::parse(&frame_data)?;
//!
//! // Extract L_Data if this is a data frame
//! if cemi.is_ldata() {
//!     let ldata = cemi.as_ldata()?;
//!
//!     if ldata.is_group_write() {
//!         let addr = ldata.destination_group().unwrap();
//!         // Process group write...
//!     }
//! }
//! ```

use crate::addressing::{GroupAddress, IndividualAddress};
use crate::error::{KnxError, Result};
use crate::protocol::constants::{CEMIMessageCode, Priority};

/// cEMI Additional Information Type codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum AdditionalInfoType {
    /// PL medium information
    PlMediumInfo = 0x01,
    /// RF medium information
    RfMediumInfo = 0x02,
    /// Busmonitor error flags
    BusmonitorErrorFlags = 0x03,
    /// Relative timestamp
    RelativeTimestamp = 0x04,
    /// Time delay
    TimeDelay = 0x05,
    /// Extended relative timestamp
    ExtendedRelativeTimestamp = 0x06,
    /// `BiBat` information
    BiBatInfo = 0x07,
}

/// Control Field 1 of `L_Data` frame
///
/// ```text
/// Bit 7: Frame Type (0=extended, 1=standard)
/// Bit 6: Reserved
/// Bit 5: Repeat (0=repeat, 1=do not repeat)
/// Bit 4: System Broadcast (0=system, 1=broadcast)
/// Bit 3-2: Priority (00=system, 01=normal, 10=urgent, 11=low)
/// Bit 1: Acknowledge Request (0=no ack, 1=ack requested)
/// Bit 0: Confirm (0=no error, 1=error)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControlField1 {
    raw: u8,
}

impl From<u8> for ControlField1 {
    #[inline(always)]
    fn from(raw: u8) -> Self {
        Self { raw }
    }
}

impl From<ControlField1> for u8 {
    #[inline(always)]
    fn from(ctrl: ControlField1) -> u8 {
        ctrl.raw
    }
}

impl ControlField1 {
    /// Get raw byte value
    #[inline(always)]
    pub const fn raw(self) -> u8 {
        self.raw
    }

    /// Check if frame is standard (true) or extended (false)
    #[inline(always)]
    pub const fn is_standard_frame(self) -> bool {
        (self.raw & 0x80) != 0
    }

    /// Check if repeat flag is set (do not repeat if true)
    #[inline(always)]
    pub const fn do_not_repeat(self) -> bool {
        (self.raw & 0x20) != 0
    }

    /// Check if this is a system broadcast
    #[inline(always)]
    pub const fn is_broadcast(self) -> bool {
        (self.raw & 0x10) != 0
    }

    /// Get priority
    #[inline(always)]
    pub const fn priority(self) -> Priority {
        Priority::from_u8((self.raw >> 2) & 0x03)
    }

    /// Check if acknowledge is requested
    #[inline(always)]
    pub const fn ack_requested(self) -> bool {
        (self.raw & 0x02) != 0
    }

    /// Check if confirm error flag is set
    #[inline(always)]
    pub const fn has_error(self) -> bool {
        (self.raw & 0x01) != 0
    }

    /// Create a new Control Field 1
    pub const fn new(
        standard_frame: bool,
        do_not_repeat: bool,
        broadcast: bool,
        priority: Priority,
        ack_requested: bool,
        has_error: bool,
    ) -> Self {
        let mut raw = 0u8;

        if standard_frame {
            raw |= 0x80;
        }
        if do_not_repeat {
            raw |= 0x20;
        }
        if broadcast {
            raw |= 0x10;
        }
        raw |= (priority.to_u8() & 0x03) << 2;
        if ack_requested {
            raw |= 0x02;
        }
        if has_error {
            raw |= 0x01;
        }

        Self { raw }
    }
}

impl Default for ControlField1 {
    #[inline]
    fn default() -> Self {
        // Standard frame, repeat allowed, broadcast, normal priority, no ack, no error
        // Pre-calculated: 0b10010100 = 0x94
        // Bit 7: 1 = standard frame
        // Bit 5: 0 = repeat allowed
        // Bit 4: 1 = broadcast
        // Bits 3-2: 01 = normal priority
        // Bit 1: 0 = no ack
        // Bit 0: 0 = no error
        Self { raw: 0x94 }
    }
}

/// Control Field 2 of `L_Data` frame
///
/// ```text
/// Bit 7: Destination Address Type (0=individual, 1=group)
/// Bit 6-4: Hop Count (0-7)
/// Bit 3-0: Extended Frame Format (0000=standard)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControlField2 {
    raw: u8,
}

impl From<u8> for ControlField2 {
    #[inline(always)]
    fn from(raw: u8) -> Self {
        Self { raw }
    }
}

impl From<ControlField2> for u8 {
    #[inline(always)]
    fn from(ctrl: ControlField2) -> u8 {
        ctrl.raw
    }
}

impl ControlField2 {
    /// Get raw byte value
    #[inline(always)]
    pub const fn raw(self) -> u8 {
        self.raw
    }

    /// Check if destination is group address (true) or individual (false)
    #[inline(always)]
    pub const fn is_group_address(self) -> bool {
        (self.raw & 0x80) != 0
    }

    /// Get hop count (0-7)
    #[inline(always)]
    pub const fn hop_count(self) -> u8 {
        (self.raw >> 4) & 0x07
    }

    /// Get extended frame format
    #[inline(always)]
    pub const fn extended_format(self) -> u8 {
        self.raw & 0x0F
    }

    /// Create a new Control Field 2
    pub const fn new(is_group: bool, hop_count: u8, extended_format: u8) -> Self {
        let mut raw = 0u8;

        if is_group {
            raw |= 0x80;
        }
        raw |= (hop_count & 0x07) << 4;
        raw |= extended_format & 0x0F;

        Self { raw }
    }
}

impl Default for ControlField2 {
    #[inline]
    fn default() -> Self {
        // Group address, hop count 6, standard format
        // Pre-calculated: 0b11100000 = 0xE0
        // Bit 7: 1 = group address
        // Bits 6-4: 110 = hop count 6
        // Bits 3-0: 0000 = standard format
        Self { raw: 0xE0 }
    }
}

/// TPCI (Transport Layer Protocol Control Information)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Tpci {
    /// Unnumbered Data Packet (UDP)
    UnnumberedData,
    /// Numbered Data Packet (NDP) with sequence number
    NumberedData {
        /// Sequence number (0-15)
        sequence: u8,
    },
    /// Unnumbered Control Packet (UCP)
    UnnumberedControl,
    /// Numbered Control Packet (NCP) with sequence number
    NumberedControl {
        /// Sequence number (0-15)
        sequence: u8,
    },
}

impl Tpci {
    /// Parse TPCI from the first byte of TPCI/APCI field
    pub const fn from_byte(byte: u8) -> Self {
        let tpci = (byte >> 6) & 0x03;
        match tpci {
            0b00 => Self::UnnumberedData,
            0b01 => {
                let sequence = (byte >> 2) & 0x0F;
                Self::NumberedData { sequence }
            }
            0b10 => Self::UnnumberedControl,
            0b11 => {
                let sequence = (byte >> 2) & 0x0F;
                Self::NumberedControl { sequence }
            }
            _ => Self::UnnumberedData, // Unreachable but needed for const
        }
    }

    /// Check if this is a data packet
    pub const fn is_data(self) -> bool {
        matches!(self, Self::UnnumberedData | Self::NumberedData { .. })
    }
}

/// APCI (Application Layer Protocol Control Information)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Apci {
    /// Group Value Read (`A_GroupValue_Read`)
    GroupValueRead,
    /// Group Value Response (`A_GroupValue_Response`)
    GroupValueResponse,
    /// Group Value Write (`A_GroupValue_Write`)
    GroupValueWrite,
    /// Individual Address Write
    IndividualAddressWrite,
    /// Individual Address Read
    IndividualAddressRead,
    /// Individual Address Response
    IndividualAddressResponse,
    /// ADC Read
    AdcRead,
    /// ADC Response
    AdcResponse,
    /// Memory Read
    MemoryRead,
    /// Memory Response
    MemoryResponse,
    /// Memory Write
    MemoryWrite,
    /// Device Descriptor Read
    DeviceDescriptorRead,
    /// Device Descriptor Response
    DeviceDescriptorResponse,
    /// Unknown/Unsupported APCI
    Unknown(u16),
}

impl Apci {
    /// Parse APCI from TPCI/APCI bytes
    ///
    /// The APCI is encoded in the lower 10 bits across two bytes:
    /// - byte1 (TPCI byte): bits 1-0 contain APCI bits 9-8
    /// - byte2 (APCI byte): bits 7-6 contain APCI bits 7-6
    ///
    /// For data values ≤6 bits, bits 5-0 of byte2 contain the actual data value.
    pub const fn from_bytes(byte1: u8, byte2: u8) -> Self {
        // Extract APCI: byte1[1:0] << 8 | byte2[7:6] << 6
        // We mask byte2 to get only the command bits (7-6), ignoring data bits (5-0)
        let apci = ((byte1 as u16 & 0x03) << 8) | (byte2 as u16 & 0xC0);

        match apci {
            0x000 => Self::GroupValueRead,
            0x040 => Self::GroupValueResponse,
            0x080 => Self::GroupValueWrite,
            0x0C0 => Self::IndividualAddressWrite,
            0x100 => Self::IndividualAddressRead,
            0x140 => Self::IndividualAddressResponse,
            0x180 => Self::AdcRead,
            0x1C0 => Self::AdcResponse,
            0x200 => Self::MemoryRead,
            0x240 => Self::MemoryResponse,
            0x280 => Self::MemoryWrite,
            0x300 => Self::DeviceDescriptorRead,
            0x340 => Self::DeviceDescriptorResponse,
            _ => Self::Unknown(apci),
        }
    }

    /// Convert APCI to u16 value
    pub const fn to_u16(self) -> u16 {
        match self {
            Self::GroupValueRead => 0x000,
            Self::GroupValueResponse => 0x040,
            Self::GroupValueWrite => 0x080,
            Self::IndividualAddressWrite => 0x0C0,
            Self::IndividualAddressRead => 0x100,
            Self::IndividualAddressResponse => 0x140,
            Self::AdcRead => 0x180,
            Self::AdcResponse => 0x1C0,
            Self::MemoryRead => 0x200,
            Self::MemoryResponse => 0x240,
            Self::MemoryWrite => 0x280,
            Self::DeviceDescriptorRead => 0x300,
            Self::DeviceDescriptorResponse => 0x340,
            Self::Unknown(val) => val,
        }
    }
}

/// cEMI `L_Data` frame
///
/// This is the most common cEMI frame type, used for transmitting
/// KNX telegrams over KNXnet/IP.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LDataFrame<'a> {
    /// Control field 1
    pub ctrl1: ControlField1,
    /// Control field 2
    pub ctrl2: ControlField2,
    /// Source address (individual)
    pub source: IndividualAddress,
    /// Destination address (individual or group)
    pub destination_raw: u16,
    /// NPDU length (data length + 1 for TPCI/APCI)
    pub npdu_length: u8,
    /// TPCI
    pub tpci: Tpci,
    /// APCI
    pub apci: Apci,
    /// Application data
    pub data: &'a [u8],
}

impl<'a> LDataFrame<'a> {
    /// Minimum size of `L_Data` frame (without additional data bytes)
    /// Control1 + Control2 + Source(2) + Dest(2) + `NPDUlen` + TPCI + APCI = 9 bytes
    pub const MIN_SIZE: usize = 9;

    /// Parse `L_Data` frame from bytes
    ///
    /// # Errors
    ///
    /// Returns error if buffer is too small or frame is invalid
    #[inline(always)]
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < Self::MIN_SIZE {
            return Err(KnxError::buffer_too_small());
        }

        let ctrl1 = ControlField1::from(data[0]);
        let ctrl2 = ControlField2::from(data[1]);
        let source = IndividualAddress::from(u16::from_be_bytes([data[2], data[3]]));
        let destination_raw = u16::from_be_bytes([data[4], data[5]]);
        let npdu_length = data[6];

        // TPCI/APCI is at least 1 byte, could be 2
        let tpci_byte = data[7];
        let tpci = Tpci::from_byte(tpci_byte);

        // For data frames, parse APCI
        let (apci, data_start) = if tpci.is_data() {
            // SAFETY: Bounds checked above - data.len() >= MIN_SIZE = 9.
            // Index 8 is within bounds (< 9). Using get_unchecked avoids redundant
            // bounds check in this hot path for KNX frame processing.
            let apci = Apci::from_bytes(tpci_byte, unsafe { *data.get_unchecked(8) });
            (apci, 9)
        } else {
            (Apci::Unknown(0), 8)
        };

        // Extract application data
        // NPDU length includes TPCI (1 byte) + APCI (1 byte) + data
        // So: data_length = npdu_length - 2
        // But for data frames, we already consumed TPCI+APCI (2 bytes)
        // data_start is at position 9 (for data frames) or 8 (for control)
        // Total frame from NPDU start: 7 (up to NPDU) + npdu_length
        let npdu_end = 7 + npdu_length as usize;

        if data.len() < npdu_end {
            return Err(KnxError::invalid_frame());
        }

        let app_data = &data[data_start..npdu_end];

        Ok(Self {
            ctrl1,
            ctrl2,
            source,
            destination_raw,
            npdu_length,
            tpci,
            apci,
            data: app_data,
        })
    }

    /// Get destination as group address (if applicable)
    #[inline]
    pub fn destination_group(&self) -> Option<GroupAddress> {
        self.ctrl2
            .is_group_address()
            .then(|| GroupAddress::from(self.destination_raw))
    }

    /// Get destination as individual address (if applicable)
    #[inline]
    pub fn destination_individual(&self) -> Option<IndividualAddress> {
        (!self.ctrl2.is_group_address()).then(|| IndividualAddress::from(self.destination_raw))
    }

    /// Check if this is a group value write
    #[inline(always)]
    pub const fn is_group_write(&self) -> bool {
        matches!(self.apci, Apci::GroupValueWrite)
    }

    /// Check if this is a group value read
    #[inline(always)]
    pub const fn is_group_read(&self) -> bool {
        matches!(self.apci, Apci::GroupValueRead)
    }

    /// Check if this is a group value response
    #[inline(always)]
    pub const fn is_group_response(&self) -> bool {
        matches!(self.apci, Apci::GroupValueResponse)
    }
}

/// cEMI Frame wrapper
///
/// Represents a complete cEMI frame with message code and payload.
#[derive(Debug)]
pub struct CEMIFrame<'a> {
    /// Message code
    pub message_code: CEMIMessageCode,
    /// Raw frame data (including message code)
    data: &'a [u8],
}

impl<'a> CEMIFrame<'a> {
    /// Minimum cEMI frame size (message code + add info length)
    pub const MIN_SIZE: usize = 2;

    /// Parse a cEMI frame from bytes
    ///
    /// # Errors
    ///
    /// Returns error if buffer is too small or message code is invalid
    #[inline(always)]
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < Self::MIN_SIZE {
            return Err(KnxError::buffer_too_small());
        }

        let message_code =
            CEMIMessageCode::from_u8(data[0]).ok_or_else(KnxError::invalid_message_code)?;

        Ok(Self { message_code, data })
    }

    /// Get the message code
    #[inline(always)]
    pub const fn message_code(&self) -> CEMIMessageCode {
        self.message_code
    }

    /// Get additional info length
    #[inline(always)]
    pub fn additional_info_length(&self) -> u8 {
        // SAFETY: parse() guarantees data.len() >= MIN_SIZE = 2 during construction.
        // Index 1 is always valid (< 2). This eliminates bounds checking for a
        // frequently called accessor method in cEMI frame processing.
        unsafe { *self.data.get_unchecked(1) }
    }

    /// Get the service information (skipping message code and additional info)
    ///
    /// This returns the `L_Data` payload for data frames.
    #[inline]
    pub fn service_info(&self) -> Result<&[u8]> {
        let add_info_len = self.additional_info_length();
        let service_start = 2 + add_info_len as usize;

        if self.data.len() < service_start {
            return Err(KnxError::invalid_frame());
        }

        Ok(&self.data[service_start..])
    }

    /// Parse as `L_Data` frame (for `L_Data.req`, `L_Data.ind`, `L_Data.con`)
    ///
    /// # Errors
    ///
    /// Returns error if this is not an `L_Data` frame or parsing fails
    pub fn as_ldata(&self) -> Result<LDataFrame<'a>> {
        match self.message_code {
            CEMIMessageCode::LDataReq | CEMIMessageCode::LDataInd | CEMIMessageCode::LDataCon => {
                // Get additional info length
                let add_info_len = self.additional_info_length();
                let service_start = 2 + add_info_len as usize;

                if self.data.len() < service_start {
                    return Err(KnxError::invalid_frame());
                }

                // Parse directly from data with correct lifetime
                LDataFrame::parse(&self.data[service_start..])
            }
            _ => Err(KnxError::invalid_message_code()),
        }
    }

    /// Check if this is an `L_Data` frame
    pub const fn is_ldata(&self) -> bool {
        matches!(
            self.message_code,
            CEMIMessageCode::LDataReq | CEMIMessageCode::LDataInd | CEMIMessageCode::LDataCon
        )
    }
}

/// Helper function to extract data value from 6-bit encoded value in APCI
///
/// For small data (≤6 bits), the value is encoded directly in the APCI byte.
/// This is common for boolean values and small integers.
pub const fn extract_6bit_value(apci_byte: u8) -> u8 {
    apci_byte & 0x3F
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_field1() {
        let ctrl = ControlField1::new(true, false, true, Priority::Normal, false, false);
        assert!(ctrl.is_standard_frame());
        assert!(!ctrl.do_not_repeat());
        assert!(ctrl.is_broadcast());
        assert_eq!(ctrl.priority(), Priority::Normal);
        assert!(!ctrl.ack_requested());
        assert!(!ctrl.has_error());
    }

    #[test]
    fn test_control_field1_default() {
        let ctrl = ControlField1::default();
        // Verify pre-calculated value is correct
        assert_eq!(ctrl.raw(), 0x94);
        // Verify semantics
        assert!(ctrl.is_standard_frame());
        assert!(!ctrl.do_not_repeat());
        assert!(ctrl.is_broadcast());
        assert_eq!(ctrl.priority(), Priority::Normal);
        assert!(!ctrl.ack_requested());
        assert!(!ctrl.has_error());
    }

    #[test]
    fn test_control_field1_raw() {
        // 0xBC = 0b10111100
        // Bit 7: 1 = standard frame
        // Bit 6: 0 = reserved
        // Bit 5: 1 = do not repeat
        // Bit 4: 1 = broadcast
        // Bit 3-2: 11 = low priority
        // Bit 1: 0 = no ack
        // Bit 0: 0 = no error
        let ctrl = ControlField1::from(0xBCu8);
        assert!(ctrl.is_standard_frame());
        assert!(ctrl.do_not_repeat()); // Bit 5 is set
        assert!(ctrl.is_broadcast());
        assert_eq!(ctrl.priority(), Priority::Low); // Bits 3-2 = 11
        assert!(!ctrl.ack_requested());
        assert!(!ctrl.has_error());
    }

    #[test]
    fn test_control_field2() {
        let ctrl = ControlField2::new(true, 6, 0);
        assert!(ctrl.is_group_address());
        assert_eq!(ctrl.hop_count(), 6);
        assert_eq!(ctrl.extended_format(), 0);
    }

    #[test]
    fn test_control_field2_default() {
        let ctrl = ControlField2::default();
        // Verify pre-calculated value is correct
        assert_eq!(ctrl.raw(), 0xE0);
        // Verify semantics
        assert!(ctrl.is_group_address());
        assert_eq!(ctrl.hop_count(), 6);
        assert_eq!(ctrl.extended_format(), 0);
    }

    #[test]
    fn test_control_field2_individual_addr() {
        let ctrl = ControlField2::new(false, 5, 0);
        assert!(!ctrl.is_group_address());
        assert_eq!(ctrl.hop_count(), 5);
    }

    #[test]
    fn test_tpci_parse() {
        let tpci = Tpci::from_byte(0b00000000);
        assert_eq!(tpci, Tpci::UnnumberedData);
        assert!(tpci.is_data());

        let tpci = Tpci::from_byte(0b01001100);
        assert!(matches!(tpci, Tpci::NumberedData { sequence: 3 }));
        assert!(tpci.is_data());

        let tpci = Tpci::from_byte(0b10000000);
        assert_eq!(tpci, Tpci::UnnumberedControl);
        assert!(!tpci.is_data());
    }

    #[test]
    fn test_apci_parse() {
        let apci = Apci::from_bytes(0x00, 0x00);
        assert_eq!(apci, Apci::GroupValueRead);

        let apci = Apci::from_bytes(0x00, 0x80);
        assert_eq!(apci, Apci::GroupValueWrite);

        let apci = Apci::from_bytes(0x00, 0x40);
        assert_eq!(apci, Apci::GroupValueResponse);
    }

    #[test]
    fn test_apci_roundtrip() {
        let apci = Apci::GroupValueWrite;
        let val = apci.to_u16();
        assert_eq!(val, 0x080);
    }

    #[test]
    fn test_ldata_frame_parse_group_write() {
        // Example: Group write to 1/2/3 with value 0x01
        let data = [
            0xBC, // Control field 1
            0xE0, // Control field 2 (group address, hop count 6)
            0x11, 0x01, // Source: 1.1.1
            0x0A, 0x03, // Destination: 1/2/3
            0x02, // NPDU length (TPCI/APCI + data = 2 bytes)
            0x00, // TPCI (unnumbered data)
            0x81, // APCI (group write) + 6-bit data (0x01)
        ];

        let frame = LDataFrame::parse(&data).unwrap();
        assert_eq!(frame.source, IndividualAddress::new(1, 1, 1).unwrap());
        assert_eq!(frame.ctrl2.is_group_address(), true);
        assert_eq!(
            frame.destination_group().unwrap(),
            GroupAddress::new(1, 2, 3).unwrap()
        );
        assert!(frame.is_group_write());
        // For 6-bit values, data is encoded in APCI byte
        // The actual data extraction would be: extract_6bit_value(0x81) = 0x01
    }

    #[test]
    fn test_ldata_frame_parse_group_read() {
        // Example: Group read from 5/6/7
        let data = [
            0xBC, // Control field 1
            0xE0, // Control field 2
            0x12, 0x05, // Source: 1.2.5
            0x2E, 0x07, // Destination: 5/6/7
            0x02, // NPDU length (TPCI + APCI = 2 bytes minimum for data frames)
            0x00, // TPCI (unnumbered data)
            0x00, // APCI (group read)
        ];

        let frame = LDataFrame::parse(&data).unwrap();
        assert!(frame.is_group_read());
        assert_eq!(
            frame.destination_group().unwrap(),
            GroupAddress::new(5, 6, 7).unwrap()
        );
    }

    #[test]
    fn test_cemi_frame_parse() {
        // Complete cEMI frame: L_Data.ind
        let data = [
            0x29, // Message code: L_Data.ind
            0x00, // Add info length (none)
            0xBC, // Control field 1
            0xE0, // Control field 2
            0x11, 0x01, // Source
            0x0A, 0x03, // Destination
            0x02, // NPDU length (2 bytes for TPCI+APCI)
            0x00, // TPCI (unnumbered data)
            0x80, // APCI (group write)
        ];

        let cemi = CEMIFrame::parse(&data).unwrap();
        assert_eq!(cemi.message_code(), CEMIMessageCode::LDataInd);
        assert_eq!(cemi.additional_info_length(), 0);
        assert!(cemi.is_ldata());

        let ldata = cemi.as_ldata().unwrap();
        assert!(ldata.is_group_write());
    }

    #[test]
    fn test_cemi_frame_with_additional_info() {
        // cEMI with additional info
        let data = [
            0x11, // Message code: L_Data.req
            0x04, // Add info length: 4 bytes
            0x01, 0x02, 0x03, 0x04, // Additional info (dummy)
            0xBC, 0xE0, 0x11, 0x01, 0x0A, 0x03, 0x01, 0x00, 0x80,
        ];

        let cemi = CEMIFrame::parse(&data).unwrap();
        assert_eq!(cemi.additional_info_length(), 4);

        let service_info = cemi.service_info().unwrap();
        assert_eq!(service_info[0], 0xBC); // Should skip to control field 1
    }

    #[test]
    fn test_extract_6bit_value() {
        assert_eq!(extract_6bit_value(0x81), 0x01);
        assert_eq!(extract_6bit_value(0xBF), 0x3F);
        assert_eq!(extract_6bit_value(0x80), 0x00);
    }

    #[test]
    fn test_cemi_invalid_message_code() {
        let data = [0xFF, 0x00]; // Invalid message code
        let result = CEMIFrame::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_ldata_buffer_too_small() {
        let data = [0xBC, 0xE0, 0x11]; // Too small
        let result = LDataFrame::parse(&data);
        assert!(result.is_err());
    }
}
