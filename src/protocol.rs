pub mod handshake;
pub mod login;
pub mod status;
pub mod play;

use std::{io::{Cursor, Read}, fmt::LowerHex};

use bytes::{BytesMut, Buf, Bytes};
use tokio_util::codec::{Decoder, Encoder};

use crate::{util::ProtocolError, io::{PacketReader, PacketWriter}};

#[derive(Debug)]
pub enum ProtocolState {
    Status,
    Login,
}

// #region packet wrappers

/// Wrapper for incoming packets
#[derive(Debug)]
pub struct PacketData {
    pub packet_id: i32,
    data: Cursor<Vec<u8>>,
}

impl PacketReader for PacketData {
    fn read_byte(&mut self) -> crate::util::Result<i8> {
        self.data.read_byte()
    }

    fn read_ubyte(&mut self) -> crate::util::Result<u8> {
        self.data.read_ubyte()
    }

    fn read_short(&mut self) -> crate::util::Result<i16> {
        self.data.read_short()
    }

    fn read_ushort(&mut self) -> crate::util::Result<u16> {
        self.data.read_ushort()
    }

    fn read_int(&mut self) -> crate::util::Result<i32> {
        self.data.read_int()
    }

    fn read_long(&mut self) -> crate::util::Result<i64> {
        self.data.read_long()
    }

    fn read_ulong(&mut self) -> crate::util::Result<u64> {
        self.data.read_ulong()
    }

    fn read_float(&mut self) -> crate::util::Result<f32> {
        self.data.read_float()
    }

    fn read_double(&mut self) -> crate::util::Result<f64> {
        self.data.read_double()
    }

    fn read_string(&mut self, max_len: i32) -> crate::util::Result<String> {
        self.data.read_string(max_len)
    }

    // We implement this differently to the blanket impl as we can optimise the Vec
    // capcacity based on the actual amount of data we have.
    fn read_remaining(&mut self) -> crate::util::Result<Vec<u8>> {
        let remaining = self.data.get_ref().len() - self.data.position() as usize;
        let mut buffer = Vec::with_capacity(remaining);
        self.data.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

impl LowerHex for PacketData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&Bytes::from(self.data.get_ref().clone()), f)
    }
}

/// Wrapper for outgoing packets
#[derive(Debug)]
pub struct PacketPayload {
    packet_id: i32,
    data: Vec<u8>,
}

impl PacketPayload {
    pub fn new(packet_id: i32) -> Self {
        Self {
            packet_id,
            data: vec![],
        }
    }

    pub fn with_capacity(packet_id: i32, capacity: usize) -> Self {
        Self {
            packet_id,
            data: Vec::with_capacity(capacity),
        }
    }
}

impl PacketWriter for PacketPayload {
    fn write_bytes(&mut self, bytes: &[u8]) -> crate::util::Result<()> {
        self.data.write_bytes(bytes)
    }

    fn write_byte(&mut self, v: i8) -> crate::util::Result<()> {
        self.data.write_byte(v)
    }

    fn write_ubyte(&mut self, v: u8) -> crate::util::Result<()> {
        self.data.write_ubyte(v)
    }

    fn write_short(&mut self, v: i16) -> crate::util::Result<()> {
        self.data.write_short(v)
    }

    fn write_ushort(&mut self, v: u16) -> crate::util::Result<()> {
        self.data.write_ushort(v)
    }

    fn write_int(&mut self, v: i32) -> crate::util::Result<()> {
        self.data.write_int(v)
    }

    fn write_ulong(&mut self, v: u64) -> crate::util::Result<()> {
        self.data.write_ulong(v)
    }

    fn write_long(&mut self, v: i64) -> crate::util::Result<()> {
        self.data.write_long(v)
    }

    fn write_float(&mut self, v: f32) -> crate::util::Result<()> {
        self.data.write_float(v)
    }

    fn write_double(&mut self, v: f64) -> crate::util::Result<()> {
        self.data.write_double(v)
    }

    fn write_string(&mut self, s: &str, max_len: i32) -> crate::util::Result<()> {
        self.data.write_string(s, max_len)
    }
}

// #endregion

pub struct MinecraftFramedCodec;

impl Decoder for MinecraftFramedCodec {
    type Item = PacketData;

    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut length_buffer = [0u8; 3];
        for i in 0..length_buffer.len() {
            let length = {
                let chunk = src.chunk();
                if chunk.len() < i + 1 {
                    return Ok(None);
                }
                length_buffer[i] = chunk[i];
                if length_buffer[i] & 128 == 128 {
                    Err(ProtocolError::NoPacket)
                } else {
                    let mut cur = Cursor::new(length_buffer);
                    cur.read_var_int()
                }
            };
            if let Ok(length) = length {
                let length = length as usize;
                if src.len() - (i + 1) >= length {
                    let data = src[i + 1..i + 1 + length].to_vec();
                    src.advance(i + 1 + length);
                    let mut data = Cursor::new(data);
                    let packet_id = data.read_var_int()?;
                    let data = PacketData {
                        packet_id,
                        data,
                    };
                    debug!("recieved {:?} from client", data);
                    return Ok(Some(data))
                } else {
                    src.reserve(src.len() - (i + 1));
                    return Ok(None);
                }
            }
        }

        error!("invalid packet header. length buffer: {:#x} full data follows: {:#x}", Bytes::from(length_buffer.to_vec()), src);
        Err(ProtocolError::VarIntTooLong)
    }
}

impl Encoder<PacketPayload> for MinecraftFramedCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: PacketPayload, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut id_data = Vec::with_capacity(5);
        id_data.write_var_int(item.packet_id)?;

        let mut length_data = Vec::with_capacity(5);
        debug!("computed packet length: {}", (id_data.len() + item.data.len()) as i32);
        length_data.write_var_int((id_data.len() + item.data.len()) as i32)?;

        dst.reserve(length_data.len() + id_data.len() + item.data.len());

        dst.extend_from_slice(&length_data);
        dst.extend_from_slice(&id_data);
        dst.extend_from_slice(&item.data);

        debug!("sent {:#x} to client", dst);

        Ok(())
    }
}
