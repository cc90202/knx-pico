//! Error types for KNX operations.

use core::fmt;

/// Result type alias for KNX operations.
pub type Result<T> = core::result::Result<T, KnxError>;

/// KNX protocol error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KnxError {
    // Protocol errors
    /// Invalid frame structure or format
    InvalidFrame,
    /// Frame checksum verification failed
    InvalidChecksum,
    /// Unsupported protocol version
    UnsupportedVersion,
    /// Unknown or unsupported service type
    UnsupportedServiceType,
    /// Frame payload is too large
    PayloadTooLarge,

    // Connection errors
    /// Connection attempt was refused by the gateway
    ConnectionRefused,
    /// Connection attempt timed out
    ConnectionTimeout,
    /// Connection attempt failed
    ConnectionFailed,
    /// Connection lost or broken
    ConnectionLost,
    /// Requested channel was not found
    ChannelNotFound,
    /// No free connection channels available
    NoFreeChannels,
    /// Connection is not established
    NotConnected,
    /// Connection is already established
    AlreadyConnected,

    // Tunneling errors
    /// Sequence number mismatch
    SequenceMismatch,
    /// Tunneling ACK failed or contains error
    TunnelingAckFailed,

    // Transport errors
    /// Failed to send data
    SendFailed,
    /// Failed to receive data
    ReceiveFailed,
    /// Provided buffer is too small
    BufferTooSmall,
    /// Socket is not bound
    NotBound,

    // Addressing errors
    /// Invalid individual address format
    InvalidIndividualAddress,
    /// Invalid group address format
    InvalidGroupAddress,
    /// Address component out of valid range
    AddressOutOfRange,

    // Datapoint Type errors
    /// Invalid DPT data format
    InvalidDptData,
    /// DPT value out of valid range
    DptValueOutOfRange,
    /// Unsupported DPT type
    UnsupportedDpt,

    // Application errors
    /// Operation is not supported
    UnsupportedOperation,
    /// Invalid operation for current state
    InvalidState,
    /// Operation timed out
    Timeout,

    // cEMI errors
    /// Invalid cEMI message code
    InvalidMessageCode,
    /// Invalid cEMI control field
    InvalidControlField,
}

impl fmt::Display for KnxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Protocol errors
            KnxError::InvalidFrame => write!(f, "Invalid KNX frame"),
            KnxError::InvalidChecksum => write!(f, "Frame checksum failed"),
            KnxError::UnsupportedVersion => write!(f, "Unsupported protocol version"),
            KnxError::UnsupportedServiceType => write!(f, "Unsupported service type"),
            KnxError::PayloadTooLarge => write!(f, "Payload too large"),

            // Connection errors
            KnxError::ConnectionRefused => write!(f, "Connection refused"),
            KnxError::ConnectionTimeout => write!(f, "Connection timeout"),
            KnxError::ConnectionFailed => write!(f, "Connection failed"),
            KnxError::ConnectionLost => write!(f, "Connection lost"),
            KnxError::ChannelNotFound => write!(f, "Channel not found"),
            KnxError::NoFreeChannels => write!(f, "No free channels"),
            KnxError::NotConnected => write!(f, "Not connected"),
            KnxError::AlreadyConnected => write!(f, "Already connected"),

            // Tunneling errors
            KnxError::SequenceMismatch => write!(f, "Sequence number mismatch"),
            KnxError::TunnelingAckFailed => write!(f, "Tunneling ACK failed"),

            // Transport errors
            KnxError::SendFailed => write!(f, "Send failed"),
            KnxError::ReceiveFailed => write!(f, "Receive failed"),
            KnxError::BufferTooSmall => write!(f, "Buffer too small"),
            KnxError::NotBound => write!(f, "Socket not bound"),

            // Addressing errors
            KnxError::InvalidIndividualAddress => write!(f, "Invalid individual address"),
            KnxError::InvalidGroupAddress => write!(f, "Invalid group address"),
            KnxError::AddressOutOfRange => write!(f, "Address out of range"),

            // Datapoint Type errors
            KnxError::InvalidDptData => write!(f, "Invalid DPT data"),
            KnxError::DptValueOutOfRange => write!(f, "DPT value out of range"),
            KnxError::UnsupportedDpt => write!(f, "Unsupported DPT"),

            // Application errors
            KnxError::UnsupportedOperation => write!(f, "Unsupported operation"),
            KnxError::InvalidState => write!(f, "Invalid state"),
            KnxError::Timeout => write!(f, "Operation timeout"),

            // cEMI errors
            KnxError::InvalidMessageCode => write!(f, "Invalid cEMI message code"),
            KnxError::InvalidControlField => write!(f, "Invalid cEMI control field"),
        }
    }
}

// Implement std::error::Error for examples and std-based applications
#[cfg(feature = "std")]
impl std::error::Error for KnxError {}
