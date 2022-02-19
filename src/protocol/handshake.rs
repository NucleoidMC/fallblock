use crate::{io::PacketReader, util::{Result, ProtocolError}};

use super::ProtocolState;

#[derive(Debug)]
pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: ProtocolState,
}

impl HandshakePacket {
    pub fn read<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        let protocol_version = rdr.read_var_int()?;
        let server_address = rdr.read_string(0xFF)?;
        let server_port = rdr.read_ushort()?;
        let next_state = rdr.read_var_int()?;
        let next_state = match next_state {
            1 => ProtocolState::Status,
            2 => ProtocolState::Login,
            v => return Err(ProtocolError::InvalidEnumValue(v)),
        };
        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}
