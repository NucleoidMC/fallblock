use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("varint too long")]
    VarIntTooLong,
    #[error("invalid utf-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("string too long: {0} (max: {1})")]
    StringTooLong(i32, i32),
    #[error("invalid enum value: {0}")]
    InvalidEnumValue(i32),
    #[error("missing handshake")]
    MissingHandshake,
    #[error("missing request")]
    MissingRequest,
    #[error("invalid packet id: {0}")]
    InvalidPacketId(i32),
    #[error("nbt error: {0}")]
    NBTError(#[from] nbt::Error),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("no packet")]
    NoPacket
}

pub type Result<T> = std::result::Result<T, ProtocolError>;

pub fn offline_mode_uuid(username: &str) -> Uuid {
    // Not technically what the vanilla server user to generate
    // offline mode uuids, but its close enough lol
    Uuid::new_v3(&Uuid::NAMESPACE_OID, username.as_bytes())
}
