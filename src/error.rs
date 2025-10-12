//! Error types for KNX operations following M-ERRORS-CANONICAL-STRUCTS guideline.
//!
//! This module provides structured error types with backtraces (when std is enabled)
//! and helper methods for error information.

use core::fmt;

#[cfg(feature = "std")]
use std::backtrace::Backtrace;

/// Result type alias for KNX operations.
pub type Result<T> = core::result::Result<T, KnxError>;

// =============================================================================
// Error Kind Enums (Internal)
// =============================================================================

/// Protocol error variants (internal)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum ProtocolErrorKind {
    InvalidFrame,
    InvalidChecksum,
    UnsupportedVersion,
    UnsupportedServiceType,
    PayloadTooLarge,
    InvalidMessageCode,
    InvalidControlField,
}

/// Connection error variants (internal)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum ConnectionErrorKind {
    Refused,
    Timeout,
    Failed,
    Lost,
    ChannelNotFound,
    NoFreeChannels,
    NotConnected,
    AlreadyConnected,
}

/// Tunneling error variants (internal)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum TunnelingErrorKind {
    SequenceMismatch,
    AckFailed,
}

/// Transport error variants (internal)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum TransportErrorKind {
    SendFailed,
    ReceiveFailed,
    BufferTooSmall,
    NotBound,
    SocketError,
}

/// Addressing error variants (internal)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum AddressingErrorKind {
    InvalidIndividualAddress,
    InvalidGroupAddress,
    OutOfRange,
}

/// DPT error variants (internal)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum DptErrorKind {
    InvalidData,
    ValueOutOfRange,
    UnsupportedType,
}

// =============================================================================
// Main Error Type
// =============================================================================

/// KNX protocol error types.
///
/// This is the main error type returned by all KNX operations.
/// It contains a backtrace (when std feature is enabled) and detailed
/// error information through helper methods.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KnxError {
    /// Protocol-related errors (frame parsing, version, etc.)
    Protocol(ProtocolError),
    /// Connection-related errors (connect, disconnect, etc.)
    Connection(ConnectionError),
    /// Tunneling-related errors (sequence, ACK, etc.)
    Tunneling(TunnelingError),
    /// Transport-related errors (socket, send, receive, etc.)
    Transport(TransportError),
    /// Addressing errors (invalid address format, etc.)
    Addressing(AddressingError),
    /// Datapoint Type errors (encoding, decoding, etc.)
    Dpt(DptError),
    /// Generic operation errors
    InvalidState,
    UnsupportedOperation,
    Timeout,
}

// =============================================================================
// Structured Error Types
// =============================================================================

/// Protocol error with optional backtrace
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ProtocolError {
    kind: ProtocolErrorKind,
    #[cfg(feature = "std")]
    backtrace: Backtrace,
}

impl ProtocolError {
    pub(crate) fn new(kind: ProtocolErrorKind) -> Self {
        Self {
            kind,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Check if this is an invalid frame error
    pub fn is_invalid_frame(&self) -> bool {
        matches!(self.kind, ProtocolErrorKind::InvalidFrame)
    }

    /// Check if this is an unsupported version error
    pub fn is_unsupported_version(&self) -> bool {
        matches!(self.kind, ProtocolErrorKind::UnsupportedVersion)
    }
}

/// Connection error with optional backtrace
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConnectionError {
    kind: ConnectionErrorKind,
    #[cfg(feature = "std")]
    backtrace: Backtrace,
}

impl ConnectionError {
    pub(crate) fn new(kind: ConnectionErrorKind) -> Self {
        Self {
            kind,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self.kind, ConnectionErrorKind::Timeout)
    }

    /// Check if connection was refused
    pub fn is_refused(&self) -> bool {
        matches!(self.kind, ConnectionErrorKind::Refused)
    }

    /// Check if connection was lost
    pub fn is_lost(&self) -> bool {
        matches!(self.kind, ConnectionErrorKind::Lost)
    }
}

/// Tunneling error with optional backtrace
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TunnelingError {
    kind: TunnelingErrorKind,
    #[cfg(feature = "std")]
    backtrace: Backtrace,
}

impl TunnelingError {
    pub(crate) fn new(kind: TunnelingErrorKind) -> Self {
        Self {
            kind,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Check if this is a sequence mismatch error
    pub fn is_sequence_mismatch(&self) -> bool {
        matches!(self.kind, TunnelingErrorKind::SequenceMismatch)
    }
}

/// Transport error with optional backtrace
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TransportError {
    kind: TransportErrorKind,
    #[cfg(feature = "std")]
    backtrace: Backtrace,
}

impl TransportError {
    pub(crate) fn new(kind: TransportErrorKind) -> Self {
        Self {
            kind,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Check if buffer is too small
    pub fn is_buffer_too_small(&self) -> bool {
        matches!(self.kind, TransportErrorKind::BufferTooSmall)
    }

    /// Check if this is a socket error
    pub fn is_socket_error(&self) -> bool {
        matches!(self.kind, TransportErrorKind::SocketError)
    }
}

/// Addressing error with optional backtrace
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AddressingError {
    kind: AddressingErrorKind,
    #[cfg(feature = "std")]
    backtrace: Backtrace,
}

impl AddressingError {
    pub(crate) fn new(kind: AddressingErrorKind) -> Self {
        Self {
            kind,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Check if address is out of range
    pub fn is_out_of_range(&self) -> bool {
        matches!(self.kind, AddressingErrorKind::OutOfRange)
    }
}

/// DPT error with optional backtrace
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DptError {
    kind: DptErrorKind,
    #[cfg(feature = "std")]
    backtrace: Backtrace,
}

impl DptError {
    pub(crate) fn new(kind: DptErrorKind) -> Self {
        Self {
            kind,
            #[cfg(feature = "std")]
            backtrace: Backtrace::capture(),
        }
    }

    /// Check if value is out of range
    pub fn is_out_of_range(&self) -> bool {
        matches!(self.kind, DptErrorKind::ValueOutOfRange)
    }
}

// =============================================================================
// Convenience Constructors for KnxError
// =============================================================================

impl KnxError {
    // Protocol errors
    #[inline]
    pub(crate) const fn invalid_frame() -> Self {
        Self::Protocol(ProtocolError { kind: ProtocolErrorKind::InvalidFrame, #[cfg(feature = "std")] backtrace: Backtrace::disabled() })
    }

    #[inline]
    pub(crate) const fn invalid_checksum() -> Self {
        Self::Protocol(ProtocolError { kind: ProtocolErrorKind::InvalidChecksum, #[cfg(feature = "std")] backtrace: Backtrace::disabled() })
    }

    #[inline]
    pub(crate) const fn unsupported_version() -> Self {
        Self::Protocol(ProtocolError { kind: ProtocolErrorKind::UnsupportedVersion, #[cfg(feature = "std")] backtrace: Backtrace::disabled() })
    }

    #[inline]
    pub(crate) const fn unsupported_service_type() -> Self {
        Self::Protocol(ProtocolError { kind: ProtocolErrorKind::UnsupportedServiceType, #[cfg(feature = "std")] backtrace: Backtrace::disabled() })
    }

    #[inline]
    pub(crate) const fn payload_too_large() -> Self {
        Self::Protocol(ProtocolError { kind: ProtocolErrorKind::PayloadTooLarge, #[cfg(feature = "std")] backtrace: Backtrace::disabled() })
    }

    // Connection errors
    pub(crate) fn connection_refused() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::Refused))
    }

    pub(crate) fn connection_timeout() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::Timeout))
    }

    pub(crate) fn connection_failed() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::Failed))
    }

    pub(crate) fn connection_lost() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::Lost))
    }

    pub(crate) fn not_connected() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::NotConnected))
    }

    pub(crate) fn channel_not_found() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::ChannelNotFound))
    }

    pub(crate) fn no_free_channels() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::NoFreeChannels))
    }

    pub(crate) fn already_connected() -> Self {
        Self::Connection(ConnectionError::new(ConnectionErrorKind::AlreadyConnected))
    }

    // Tunneling errors
    pub(crate) fn sequence_mismatch() -> Self {
        Self::Tunneling(TunnelingError::new(TunnelingErrorKind::SequenceMismatch))
    }

    pub(crate) fn tunneling_ack_failed() -> Self {
        Self::Tunneling(TunnelingError::new(TunnelingErrorKind::AckFailed))
    }

    // Transport errors
    pub(crate) fn buffer_too_small() -> Self {
        Self::Transport(TransportError::new(TransportErrorKind::BufferTooSmall))
    }

    pub(crate) fn socket_error() -> Self {
        Self::Transport(TransportError::new(TransportErrorKind::SocketError))
    }

    pub(crate) fn send_failed() -> Self {
        Self::Transport(TransportError::new(TransportErrorKind::SendFailed))
    }

    pub(crate) fn receive_failed() -> Self {
        Self::Transport(TransportError::new(TransportErrorKind::ReceiveFailed))
    }

    pub(crate) fn not_bound() -> Self {
        Self::Transport(TransportError::new(TransportErrorKind::NotBound))
    }

    // Addressing errors
    pub(crate) fn invalid_group_address() -> Self {
        Self::Addressing(AddressingError::new(AddressingErrorKind::InvalidGroupAddress))
    }

    pub(crate) fn invalid_address() -> Self {
        Self::Addressing(AddressingError::new(AddressingErrorKind::InvalidGroupAddress))
    }

    pub(crate) fn invalid_individual_address() -> Self {
        Self::Addressing(AddressingError::new(AddressingErrorKind::InvalidIndividualAddress))
    }

    pub(crate) fn address_out_of_range() -> Self {
        Self::Addressing(AddressingError::new(AddressingErrorKind::OutOfRange))
    }

    // DPT errors
    pub(crate) fn invalid_dpt_data() -> Self {
        Self::Dpt(DptError::new(DptErrorKind::InvalidData))
    }

    pub(crate) fn dpt_value_out_of_range() -> Self {
        Self::Dpt(DptError::new(DptErrorKind::ValueOutOfRange))
    }

    pub(crate) fn unsupported_dpt() -> Self {
        Self::Dpt(DptError::new(DptErrorKind::UnsupportedType))
    }

    // cEMI errors (Protocol category)
    pub(crate) fn invalid_message_code() -> Self {
        Self::Protocol(ProtocolError::new(ProtocolErrorKind::InvalidMessageCode))
    }

    pub(crate) fn invalid_control_field() -> Self {
        Self::Protocol(ProtocolError::new(ProtocolErrorKind::InvalidControlField))
    }
}

// =============================================================================
// Display Implementation
// =============================================================================

impl fmt::Display for KnxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KnxError::Protocol(e) => write!(f, "Protocol error: {:?}", e.kind),
            KnxError::Connection(e) => write!(f, "Connection error: {:?}", e.kind),
            KnxError::Tunneling(e) => write!(f, "Tunneling error: {:?}", e.kind),
            KnxError::Transport(e) => write!(f, "Transport error: {:?}", e.kind),
            KnxError::Addressing(e) => write!(f, "Addressing error: {:?}", e.kind),
            KnxError::Dpt(e) => write!(f, "DPT error: {:?}", e.kind),
            KnxError::InvalidState => write!(f, "Invalid state"),
            KnxError::UnsupportedOperation => write!(f, "Unsupported operation"),
            KnxError::Timeout => write!(f, "Operation timeout"),
        }

        // Note: Backtrace will be printed when std::error::Error::source() is called
    }
}

// Implement std::error::Error for std-based applications
#[cfg(feature = "std")]
impl std::error::Error for KnxError {}
