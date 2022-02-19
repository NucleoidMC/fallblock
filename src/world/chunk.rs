use serde::Serialize;

use crate::{io::PacketWriter, util::Result};

use super::{map_template::BlockState, packed_array::PackedBitArray, block_ids};

#[derive(Clone, Debug)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub sections: Vec<ChunkSection>,
}

impl Chunk {
    pub fn write<W: PacketWriter>(&self, wr: &mut W) -> Result<()> {
        for section in &self.sections {
            section.write(wr)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ChunkSection {
    pub y_pos: i32,
    pub block_count: u16,
    pub block_states: Vec<BlockState>,
}

impl ChunkSection {
    fn build_palette_data(&self) -> (Vec<i32>, PackedBitArray) {
        let mut palette = Vec::new();
        let mut states = Vec::new();

        for block in &self.block_states {
            let state_id = block_ids::get_state_id(block).expect("missing state ID");
            let index = if let Some(idx) = palette.iter().position(|s| *s == state_id) {
                idx
            } else {
                palette.push(state_id);
                palette.len() - 1
            };
            states.push(index as u64);
        }

        let mut packed_states = PackedBitArray::empty(palette.len());
        for (index, value) in states.into_iter().enumerate() {
            packed_states.put_value(index, value);
        }

        (palette, packed_states)
    }

    pub fn write<W: PacketWriter>(&self, wr: &mut W) -> Result<()> {
        wr.write_ushort(self.block_count)?;

        let (palette, states) = self.build_palette_data();
        wr.write_ubyte(states.bits_per_entry() as u8)?;

        wr.write_var_int(palette.len() as i32)?;
        for entry in &palette {
            wr.write_var_int(*entry)?;
        }
        wr.write_var_int(states.data().len() as i32)?;
        for v in states.data() {
            wr.write_ulong(*v)?;
        }

        // Biomes
        wr.write_ubyte(0)?;
        wr.write_var_int(0)?;
        wr.write_var_int(0)?;
        
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Heightmaps {
    #[serde(rename = "MOTION_BLOCKING")]
    pub motion_blocking: Vec<i64>,
}
