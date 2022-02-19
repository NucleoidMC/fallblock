use serde::Deserialize;

use crate::{io::PacketWriter, util::Result};

pub const PROTOCOL_VERSION: i32 = 757;

#[derive(Clone, Debug, Deserialize)]
pub enum Gamemode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Gamemode {
    pub fn write<W: PacketWriter>(&self, wr: &mut W) -> Result<()> {
        wr.write_ubyte(match self {
            Gamemode::Survival => 0,
            Gamemode::Creative => 1,
            Gamemode::Adventure => 2,
            Gamemode::Spectator => 3,
        })
    }
}
