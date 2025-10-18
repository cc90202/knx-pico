//! KNXnet/IP protocol constants and service type identifiers.

/// KNXnet/IP protocol version 1.0
pub const KNXNETIP_VERSION_10: u8 = 0x10;

/// Standard KNXnet/IP header length (6 bytes)
pub const HEADER_SIZE_10: u8 = 0x06;

/// Standard UDP port for KNXnet/IP communication
pub const KNXNETIP_DEFAULT_PORT: u16 = 3671;

/// Maximum size of a KNXnet/IP frame
pub const MAX_FRAME_SIZE: usize = 256;

/// Maximum size of cEMI frame payload
pub const MAX_CEMI_SIZE: usize = 64;

/// KNXnet/IP multicast address for routing
pub const KNXNETIP_MULTICAST_ADDR: &str = "224.0.23.12";

/// System Setup Multicast Address
pub const SYSTEM_SETUP_MULTICAST: &str = "224.0.23.11";

// =============================================================================
// Service Type Identifiers
// =============================================================================

// Service type constants (for convenience)
/// Service type constant for CONNECT_REQUEST (0x0205)
pub const SERVICE_CONNECT_REQUEST: u16 = 0x0205;
/// Service type constant for CONNECT_RESPONSE (0x0206)
pub const SERVICE_CONNECT_RESPONSE: u16 = 0x0206;
/// Service type constant for CONNECTIONSTATE_REQUEST (0x0207)
pub const SERVICE_CONNECTIONSTATE_REQUEST: u16 = 0x0207;
/// Service type constant for CONNECTIONSTATE_RESPONSE (0x0208)
pub const SERVICE_CONNECTIONSTATE_RESPONSE: u16 = 0x0208;
/// Service type constant for DISCONNECT_REQUEST (0x0209)
pub const SERVICE_DISCONNECT_REQUEST: u16 = 0x0209;
/// Service type constant for DISCONNECT_RESPONSE (0x020A)
pub const SERVICE_DISCONNECT_RESPONSE: u16 = 0x020A;
/// Service type constant for TUNNELING_REQUEST (0x0420)
pub const SERVICE_TUNNELING_REQUEST: u16 = 0x0420;
/// Service type constant for TUNNELING_ACK (0x0421)
pub const SERVICE_TUNNELING_ACK: u16 = 0x0421;

/// KNXnet/IP Core Service Type Identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u16)]
pub enum ServiceType {
    // Core services (0x02xx)
    /// `SEARCH_REQUEST` - Device discovery request
    SearchRequest = 0x0201,
    /// `SEARCH_RESPONSE` - Device discovery response
    SearchResponse = 0x0202,
    /// `DESCRIPTION_REQUEST` - Device description request
    DescriptionRequest = 0x0203,
    /// `DESCRIPTION_RESPONSE` - Device description response
    DescriptionResponse = 0x0204,
    /// `CONNECT_REQUEST` - Connection request
    ConnectRequest = 0x0205,
    /// `CONNECT_RESPONSE` - Connection response
    ConnectResponse = 0x0206,
    /// `CONNECTIONSTATE_REQUEST` - Connection state request (heartbeat)
    ConnectionstateRequest = 0x0207,
    /// `CONNECTIONSTATE_RESPONSE` - Connection state response
    ConnectionstateResponse = 0x0208,
    /// `DISCONNECT_REQUEST` - Disconnect request
    DisconnectRequest = 0x0209,
    /// `DISCONNECT_RESPONSE` - Disconnect response
    DisconnectResponse = 0x020A,

    // Device Management (0x03xx)
    /// `DEVICE_CONFIGURATION_REQUEST`
    DeviceConfigurationRequest = 0x0310,
    /// `DEVICE_CONFIGURATION_ACK`
    DeviceConfigurationAck = 0x0311,

    // Tunnelling (0x04xx)
    /// `TUNNELLING_REQUEST` - Tunnelling data request
    TunnellingRequest = 0x0420,
    /// `TUNNELLING_ACK` - Tunnelling acknowledgement
    TunnellingAck = 0x0421,

    // Routing (0x05xx)
    /// `ROUTING_INDICATION` - Routing indication (multicast)
    RoutingIndication = 0x0530,
    /// `ROUTING_LOST_MESSAGE` - Routing lost message indication
    RoutingLostMessage = 0x0531,
    /// `ROUTING_BUSY` - Routing busy indication
    RoutingBusy = 0x0532,

    // Remote Logging (0x06xx)
    /// `REMOTE_DIAGNOSTIC_REQUEST`
    RemoteDiagnosticRequest = 0x0740,
    /// `REMOTE_DIAGNOSTIC_RESPONSE`
    RemoteDiagnosticResponse = 0x0741,

    // Secure services (0x09xx)
    /// `SECURE_WRAPPER` - Secure session wrapper
    SecureWrapper = 0x0950,
    /// `SESSION_REQUEST` - Secure session request
    SessionRequest = 0x0951,
    /// `SESSION_RESPONSE` - Secure session response
    SessionResponse = 0x0952,
    /// `SESSION_AUTHENTICATE` - Secure session authentication
    SessionAuthenticate = 0x0953,
    /// `SESSION_STATUS` - Secure session status
    SessionStatus = 0x0954,
}

impl ServiceType {
    /// Convert a u16 to `ServiceType`
    pub const fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0201 => Some(Self::SearchRequest),
            0x0202 => Some(Self::SearchResponse),
            0x0203 => Some(Self::DescriptionRequest),
            0x0204 => Some(Self::DescriptionResponse),
            0x0205 => Some(Self::ConnectRequest),
            0x0206 => Some(Self::ConnectResponse),
            0x0207 => Some(Self::ConnectionstateRequest),
            0x0208 => Some(Self::ConnectionstateResponse),
            0x0209 => Some(Self::DisconnectRequest),
            0x020A => Some(Self::DisconnectResponse),
            0x0310 => Some(Self::DeviceConfigurationRequest),
            0x0311 => Some(Self::DeviceConfigurationAck),
            0x0420 => Some(Self::TunnellingRequest),
            0x0421 => Some(Self::TunnellingAck),
            0x0530 => Some(Self::RoutingIndication),
            0x0531 => Some(Self::RoutingLostMessage),
            0x0532 => Some(Self::RoutingBusy),
            0x0740 => Some(Self::RemoteDiagnosticRequest),
            0x0741 => Some(Self::RemoteDiagnosticResponse),
            0x0950 => Some(Self::SecureWrapper),
            0x0951 => Some(Self::SessionRequest),
            0x0952 => Some(Self::SessionResponse),
            0x0953 => Some(Self::SessionAuthenticate),
            0x0954 => Some(Self::SessionStatus),
            _ => None,
        }
    }

    /// Convert `ServiceType` to u16
    pub const fn to_u16(self) -> u16 {
        self as u16
    }
}

// =============================================================================
// Connection Type Codes
// =============================================================================

/// Connection type for `DEVICE_MGMT_CONNECTION`
pub const DEVICE_MGMT_CONNECTION: u8 = 0x03;

/// Connection type for `TUNNEL_CONNECTION`
pub const TUNNEL_CONNECTION: u8 = 0x04;

/// Connection type for `REMLOG_CONNECTION`
pub const REMLOG_CONNECTION: u8 = 0x06;

/// Connection type for `REMCONF_CONNECTION`
pub const REMCONF_CONNECTION: u8 = 0x07;

/// Connection type for `OBJSVR_CONNECTION`
pub const OBJSVR_CONNECTION: u8 = 0x08;

// =============================================================================
// Host Protocol Codes
// =============================================================================

/// IPv4 UDP protocol
pub const IPV4_UDP: u8 = 0x01;

/// IPv4 TCP protocol
pub const IPV4_TCP: u8 = 0x02;

// =============================================================================
// Error Codes
// =============================================================================

/// Error code for successful operation
pub const E_NO_ERROR: u8 = 0x00;

/// Error code for connection type not supported
pub const E_CONNECTION_TYPE: u8 = 0x22;

/// Error code for connection option not supported
pub const E_CONNECTION_OPTION: u8 = 0x23;

/// Error code for no more connections available
pub const E_NO_MORE_CONNECTIONS: u8 = 0x24;

/// Error code for data connection error
pub const E_DATA_CONNECTION: u8 = 0x26;

/// Error code for KNX connection error
pub const E_KNX_CONNECTION: u8 = 0x27;

/// Error code for tunnelling layer not supported
pub const E_TUNNELLING_LAYER: u8 = 0x29;

// =============================================================================
// cEMI Message Codes
// =============================================================================

/// cEMI Message Codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum CEMIMessageCode {
    /// `L_Raw.req` - Raw frame request
    LRawReq = 0x10,
    /// `L_Data.req` - Data request
    LDataReq = 0x11,
    /// `L_Poll_Data.req` - Poll data request
    LPollDataReq = 0x13,
    /// `L_Raw.ind` - Raw frame indication
    LRawInd = 0x2D,
    /// `L_Data.ind` - Data indication
    LDataInd = 0x29,
    /// `L_Busmon.ind` - Bus monitor indication
    LBusmonInd = 0x2B,
    /// `L_Raw.con` - Raw frame confirmation
    LRawCon = 0x2F,
    /// `L_Data.con` - Data confirmation
    LDataCon = 0x2E,
    /// `L_Poll_Data.con` - Poll data confirmation
    LPollDataCon = 0x25,
}

impl CEMIMessageCode {
    /// Convert u8 to `CEMIMessageCode`
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x10 => Some(Self::LRawReq),
            0x11 => Some(Self::LDataReq),
            0x13 => Some(Self::LPollDataReq),
            0x2D => Some(Self::LRawInd),
            0x29 => Some(Self::LDataInd),
            0x2B => Some(Self::LBusmonInd),
            0x2F => Some(Self::LRawCon),
            0x2E => Some(Self::LDataCon),
            0x25 => Some(Self::LPollDataCon),
            _ => None,
        }
    }

    /// Convert `CEMIMessageCode` to u8
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

// =============================================================================
// KNX Priority
// =============================================================================

/// KNX message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum Priority {
    /// System priority
    System = 0b00,
    /// Normal priority (default)
    Normal = 0b01,
    /// Urgent priority
    Urgent = 0b10,
    /// Low priority
    Low = 0b11,
}

impl Priority {
    /// Convert u8 to Priority
    pub const fn from_u8(value: u8) -> Self {
        match value & 0b11 {
            0b00 => Self::System,
            0b01 => Self::Normal,
            0b10 => Self::Urgent,
            0b11 => Self::Low,
            _ => Self::Normal, // unreachable, but needed for exhaustiveness
        }
    }

    /// Convert Priority to u8
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}
