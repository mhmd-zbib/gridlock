/// Errors that can occur while decoding a datagram.
#[derive(Debug)]
pub enum DecodeError {
    /// The buffer ended before all expected bytes were read.
    UnexpectedEof,
    /// The first byte does not correspond to any known [`PacketKind`].
    UnknownKind(u8),
    /// A field value was out of range or otherwise invalid.
    InvalidField(&'static str),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of datagram"),
            Self::UnknownKind(k) => write!(f, "unknown packet kind: 0x{k:02x}"),
            Self::InvalidField(name) => write!(f, "invalid value in field `{name}`"),
        }
    }
}

impl std::error::Error for DecodeError {}
