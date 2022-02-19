use std::{sync::{atomic::{Ordering, AtomicI32}, Arc}, collections::HashMap};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{config::Config, world::{map_template::{MapTemplate, BlockEntity}, chunk::Chunk}};

#[derive(Clone, Debug)]
pub struct ServerStore(Arc<StoreData>);

#[derive(Debug)]
struct StoreData {
    config: Config,
    chunks: Vec<Chunk>,
    block_entities: Vec<BlockEntity>,
    next_player_id: AtomicI32,
    player_id_map: RwLock<HashMap<Uuid, i32>>,
}

impl ServerStore {
    pub fn new(config: Config, map: MapTemplate) -> Self {
        Self(Arc::new(StoreData {
            config,
            block_entities: map.block_entities.clone(),
            chunks: map.into_chunks(),
            next_player_id: AtomicI32::new(0),
            player_id_map: RwLock::new(HashMap::new()),
        }))
    }

    pub fn get_config(&self) -> &Config {
        &self.0.config
    }

    pub fn get_chunks(&self) -> &[Chunk] {
        &self.0.chunks
    }

    pub fn get_block_entities(&self) -> &[BlockEntity] {
        &self.0.block_entities
    }

    pub async fn get_player_id(&self, uuid: Uuid) -> i32 {
        let id = self.0.player_id_map.read().await.get(&uuid).cloned();
        if let Some(id) = id {
            return id;
        }
        let id = self.0.next_player_id.fetch_add(1, Ordering::Relaxed);
        self.0.player_id_map.write().await.insert(uuid, id);
        id
    }
}
