use nbt::Map;
use serde::Deserialize;

use super::map_template::BlockState;

lazy_static::lazy_static! {
    static ref BLOCK_DATA: Map<String, Block> = {
        const BLOCKS: &str = include_str!("blocks.json");
        serde_json::from_str(BLOCKS).expect("failed to parse blocks.json")
    };

    static ref BLOCK_ENTITY_DATA: Map<String, i32> = {
        const BLOCK_ENTITIES: &str = include_str!("block_entities.json");
        serde_json::from_str(BLOCK_ENTITIES).expect("failed to parse block_entities.json")
    };
}

#[derive(Deserialize)]
struct Block {
    states: Vec<BlockStateId>,
}

#[derive(Deserialize)]
struct BlockStateId {
    properties: Option<Map<String, String>>,
    id: i32,
    #[serde(default)]
    #[allow(unused)] // shut
    default: bool,
}

pub fn get_state_id(blockstate: &BlockState) -> Option<i32> {
    let block = BLOCK_DATA.get(&blockstate.name);
    if let Some(block) = block {
        for state in &block.states {
            if state.properties == blockstate.properties {
                return Some(state.id);
            }
        }
    }
    None
}

pub fn get_block_entity_id(be: &str) -> Option<i32> {
    BLOCK_ENTITY_DATA.get(be).cloned()
}
