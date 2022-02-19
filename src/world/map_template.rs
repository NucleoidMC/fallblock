use std::{path::Path, fs::File};

use nbt::{Map, Value};
use serde::Deserialize;

use crate::util::Result;

use super::{chunk::{Chunk, ChunkSection}, packed_array::PackedBitArray};

#[derive(Debug, Deserialize)]
pub struct MapTemplate {
    pub block_entities: Vec<BlockEntity>,
    pub biome: String,
    pub chunks: Vec<TemplateChunk>,
}

impl MapTemplate {
    pub fn into_chunks(self) -> Vec<Chunk> {
        let mut chunks = Map::<(i32, i32), Map<i32, ChunkSection>>::new();

        for chunk in self.chunks {
            let (x, y, z) = chunk.pos;
            chunks.entry((x, z)).or_insert(Map::new()).insert(y, chunk.into());
        }

        let mut completed_chunks = Vec::with_capacity(chunks.len());

        for ((x, z), sections) in chunks {
            let mut full_sections = Vec::with_capacity(16);

            for y in 0..16 {
                let section = sections.get(&y).cloned().unwrap_or_else(|| create_empty_section(y));
                full_sections.push(section);
            }

            completed_chunks.push(Chunk {
                x,
                z,
                sections: full_sections,
            });
        }

        completed_chunks.sort_by_key(|c| i32::abs(c.x * 256 + c.z));

        completed_chunks
    }
}

fn create_empty_section(y: i32) -> ChunkSection {
    ChunkSection {
        y_pos: y,
        block_count: 0,
        block_states: vec![BlockState {
            name: "minecraft:air".to_string(),
            properties: None,
        }; 4096],
    }
}

#[derive(Debug, Deserialize)]
pub struct TemplateChunk {
    pub block_states: BlockStates,
    pub pos: (i32, i32, i32),
}

impl Into<ChunkSection> for TemplateChunk {
    fn into(self) -> ChunkSection {
        let packed_states = PackedBitArray::new(self.block_states.data, self.block_states.palette.len());

        let mut block_states = Vec::new();

        for i in 0..4096 {
            let v = packed_states.get_value(i);
            let state = self.block_states.palette.get(v as usize).unwrap();
            block_states.push(state.clone());
        }

        ChunkSection {
            y_pos: self.pos.1,
            block_count: 4096,
            block_states,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BlockStates {
    data: Vec<u64>,
    palette: Vec<BlockState>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct BlockState {
    pub name: String,
    pub properties: Option<Map<String, String>>,
}

pub fn load_template(path: &Path) -> Result<MapTemplate> {
    let mut file = File::open(path)?;
    let template: MapTemplate = nbt::from_gzip_reader(&mut file)?;
    Ok(template)
}

#[derive(Clone, Debug, Deserialize)]
pub struct BlockEntity {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    #[serde(flatten)]
    pub data: Map<String, Value>,
}
