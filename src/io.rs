use byteorder::BigEndian;
use uuid::Uuid;

use crate::util::{Result, ProtocolError};

// I am eternally greatful to the wonderful folk who maintain wiki.vg
// The varint/varlong code is taken directly from there: https://wiki.vg/Protocol#VarInt_and_VarLong

pub trait PacketReader: std::fmt::Debug {
    fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_byte()? != 0)
    }

    fn read_byte(&mut self) -> Result<i8>;
    fn read_ubyte(&mut self) -> Result<u8>;

    fn read_short(&mut self) -> Result<i16>;
    fn read_ushort(&mut self) -> Result<u16>;

    fn read_int(&mut self) -> Result<i32>;
    fn read_long(&mut self) -> Result<i64>;
    fn read_ulong(&mut self) -> Result<u64>;

    fn read_float(&mut self) -> Result<f32>;
    fn read_double(&mut self) -> Result<f64>;

    fn read_string(&mut self, max_len: i32) -> Result<String>;

    fn read_var_int(&mut self) -> Result<i32> {
        let mut value = 0;
        let mut length = 0;
        loop {
            let current_byte = self.read_ubyte()?;
            value |= ((current_byte & 0x7F) as i32) << (length * 7);
            length += 1;
            if length > 5 {
                return Err(ProtocolError::VarIntTooLong);
            }
            if current_byte & 0x80 != 0x80 {
                break;
            }
        }
        Ok(value)
    }

    fn read_var_long(&mut self) -> Result<i64> {
        let mut value = 0;
        let mut length = 0;
        loop {
            let current_byte = self.read_ubyte()?;
            value |= ((current_byte & 0x7F) as i64) << (length * 7);
            length += 1;
            if length > 5 {
                return Err(ProtocolError::VarIntTooLong);
            }
            if (current_byte & 0x80) != 0x80 {
                break;
            }
        }
        Ok(value)
    }

    fn read_remaining(&mut self) -> Result<Vec<u8>>;

    fn read_uuid(&mut self) -> Result<Uuid> {
        let msb = self.read_ulong()?;
        let lsb = self.read_ulong()?;
        let data = (msb as u128) << 64 | (lsb as u128);
        Ok(Uuid::from_u128(data))
    }
}

impl<T: byteorder::ReadBytesExt + std::fmt::Debug> PacketReader for T {
    fn read_byte(&mut self) -> Result<i8> {
        Ok(self.read_i8()?)
    }

    fn read_ubyte(&mut self) -> Result<u8> {
        Ok(self.read_u8()?)
    }

    fn read_short(&mut self) -> Result<i16> {
        Ok(self.read_i16::<BigEndian>()?)
    }

    fn read_ushort(&mut self) -> Result<u16> {
        Ok(self.read_u16::<BigEndian>()?)
    }

    fn read_int(&mut self) -> Result<i32> {
        Ok(self.read_i32::<BigEndian>()?)
    }

    fn read_long(&mut self) -> Result<i64> {
        Ok(self.read_i64::<BigEndian>()?)
    }

    fn read_ulong(&mut self) -> Result<u64> {
        Ok(self.read_u64::<BigEndian>()?)
    }

    fn read_float(&mut self) -> Result<f32> {
        Ok(self.read_f32::<BigEndian>()?)
    }

    fn read_double(&mut self) -> Result<f64> {
        Ok(self.read_f64::<BigEndian>()?)
    }

    fn read_string(&mut self, max_len: i32) -> Result<String> {
        let length = self.read_var_int()?;
        if length > max_len {
            return Err(ProtocolError::StringTooLong(length, max_len));
        }
        let mut buffer = vec![0; length as usize];
        self.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    fn read_remaining(&mut self) -> Result<Vec<u8>> {
        let mut buffer = vec![];
        self.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}

pub trait PacketWriter {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()>;

    fn write_bool(&mut self, b: bool) -> Result<()> {
        self.write_byte(if b { 1 } else { 0 })
    }

    fn write_byte(&mut self, v: i8) -> Result<()>;
    fn write_ubyte(&mut self, v: u8) -> Result<()>;

    fn write_short(&mut self, v: i16) -> Result<()>;
    fn write_ushort(&mut self, v: u16) -> Result<()>;

    fn write_int(&mut self, v: i32) -> Result<()>;
    fn write_ulong(&mut self, v: u64) -> Result<()>;
    fn write_long(&mut self, v: i64) -> Result<()>;

    fn write_float(&mut self, v: f32) -> Result<()>;
    fn write_double(&mut self, v: f64) -> Result<()>;

    fn write_string(&mut self, s: &str, max_len: i32) -> Result<()>;

    fn write_var_int(&mut self, v: i32) -> Result<()> {
        let mut v = v as u32;
        loop {
            if (v & !0x7F) == 0 {
                return self.write_ubyte(v as u8);
            }
            self.write_ubyte((v as u8 & 0x7F) | 0x80)?;
            v >>= 7;
        }
    }

    fn write_var_long(&mut self, v: i64) -> Result<()> {
        let mut v = v;
        loop {
            if (v & !0x7F) == 0 {
                return self.write_ubyte(v as u8);
            }
            self.write_ubyte((v as u8 & 0x7F) | 0x80)?;
            v >>= 7;
        }
    }

    fn write_string_arr(&mut self, arr: &[String]) -> Result<()> {
        self.write_var_int(arr.len() as i32)?;
        for s in arr {
            self.write_string(s, 32767)?;
        }
        Ok(())
    }

    fn write_ulong_array(&mut self, arr: &[u64]) -> Result<()> {
        self.write_var_int(arr.len() as i32)?;
        for v in arr {
            self.write_ulong(*v)?;
        }
        Ok(())
    }

    fn write_uuid(&mut self, uuid: &Uuid) -> Result<()> {
        let data = uuid.as_u128();
        let msb = (data >> 64) as u64;
        let lsb = (data & 0xFFFFFFFFFFFFFFFF) as u64;
        self.write_ulong(msb)?;
        self.write_ulong(lsb)?;
        Ok(())
    }

    fn write_nbt<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<()> {
        let mut out = vec![];
        nbt::to_writer(&mut out, value, None)?;
        self.write_bytes(&out)?;
        Ok(())
    }

    fn write_position(&mut self, x: i32, y: i32, z: i32) -> Result<()> {
        let x = (x as u64) & 0x3FFFFFF;
        let y = (y as u64) & 0x3FFFFFF;
        let z = (z as u64) & 0xFFF;
        self.write_ulong(x << 38 | z << 12 | y)?;
        Ok(())
    }

    fn write_json<T: serde::Serialize>(&mut self, v: &T) -> Result<()> {
        let json = serde_json::to_string(v)?;
        self.write_string(&json, 32767)
    }
}

impl<T: byteorder::WriteBytesExt> PacketWriter for T {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        Ok(self.write_all(bytes)?)
    }

    fn write_byte(&mut self, v: i8) -> Result<()> {
        Ok(self.write_i8(v)?)
    }

    fn write_ubyte(&mut self, v: u8) -> Result<()> {
        Ok(self.write_u8(v)?)
    }

    fn write_short(&mut self, v: i16) -> Result<()> {
        Ok(self.write_i16::<BigEndian>(v)?)
    }

    fn write_ushort(&mut self, v: u16) -> Result<()> {
        Ok(self.write_u16::<BigEndian>(v)?)
    }

    fn write_int(&mut self, v: i32) -> Result<()> {
        Ok(self.write_i32::<BigEndian>(v)?)
    }

    fn write_ulong(&mut self, v: u64) -> Result<()> {
        Ok(self.write_u64::<BigEndian>(v)?)
    }

    fn write_long(&mut self, v: i64) -> Result<()> {
        Ok(self.write_i64::<BigEndian>(v)?)
    }

    fn write_float(&mut self, v: f32) -> Result<()> {
        Ok(self.write_f32::<BigEndian>(v)?)
    }

    fn write_double(&mut self, v: f64) -> Result<()> {
        Ok(self.write_f64::<BigEndian>(v)?)
    }

    fn write_string(&mut self, s: &str, max_len: i32) -> Result<()> {
        let bytes = s.as_bytes();
        let length = bytes.len() as i32;
        if length > max_len {
            Err(ProtocolError::StringTooLong(length, max_len))
        } else {
            self.write_var_int(length)?;
            self.write_all(bytes)?;
            Ok(())
        }
    }
}
