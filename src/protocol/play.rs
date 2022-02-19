use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::{Sink, SinkExt, TryStream, TryStreamExt};
use serde::Deserialize;
use tokio::time::interval;
use uuid::Uuid;

use crate::{
    constants::Gamemode,
    io::{PacketReader, PacketWriter},
    store::ServerStore,
    util::{ProtocolError, Result},
    world::{
        chunk::{Chunk, Heightmaps},
        dimension::{DimensionCodec, DimensionType}, map_template::BlockEntity, block_ids,
    },
};

use super::{PacketData, PacketPayload};

// TODO: This file should probably be split up a bit.

#[derive(Debug)]
pub enum IncomingPlayPacket {
    TeleportConfirm {
        teleport_id: i32,
    },
    ClientSettings {
        locale: String,
        view_distance: i8,
        chat_mode: i32,
        chat_colours: bool,
        displayed_skin_parts: i8,
        main_hand: i32,
        enable_text_filtering: bool,
        allow_server_listings: bool,
    },
    CustomPayload(PlayCustomPayload),
    KeepAlive(i64),
    PlayerPosition {
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
    },
    PlayerPositionAndRotation {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    PlayerRotation {
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
}

impl IncomingPlayPacket {
    pub fn read<R: PacketReader>(packet_id: i32, rdr: &mut R) -> Result<Option<Self>> {
        match packet_id {
            0x00 => Ok(Some(Self::read_teleport_confirm(rdr)?)),
            0x05 => Ok(Some(Self::read_client_settings(rdr)?)),
            0x0A => Ok(PlayCustomPayload::read(rdr)?.map(|p| Self::CustomPayload(p))),
            0x0F => Ok(Some(Self::KeepAlive(rdr.read_long()?))),
            0x11 => Ok(Some(Self::read_player_position(rdr)?)),
            0x12 => Ok(Some(Self::read_player_position_and_rotation(rdr)?)),
            0x13 => Ok(Some(Self::read_player_rotation(rdr)?)),
            _ => Ok(None),
        }
    }

    fn read_teleport_confirm<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::TeleportConfirm {
             teleport_id: rdr.read_var_int()?,
        })
    }

    fn read_client_settings<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::ClientSettings {
            locale: rdr.read_string(16)?,
            view_distance: rdr.read_byte()?,
            chat_mode: rdr.read_var_int()?,
            chat_colours: rdr.read_bool()?,
            displayed_skin_parts: rdr.read_byte()?,
            main_hand: rdr.read_var_int()?,
            enable_text_filtering: rdr.read_bool()?,
            allow_server_listings: rdr.read_bool()?,
        })
    }

    fn read_player_position<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::PlayerPosition {
            x: rdr.read_double()?,
            y: rdr.read_double()?,
            z: rdr.read_double()?,
            on_ground: rdr.read_bool()?,
        })
    }

    fn read_player_position_and_rotation<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::PlayerPositionAndRotation {
            x: rdr.read_double()?,
            y: rdr.read_double()?,
            z: rdr.read_double()?,
            yaw: rdr.read_float()?,
            pitch: rdr.read_float()?,
            on_ground: rdr.read_bool()?,
        })
    }

    fn read_player_rotation<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::PlayerRotation {
            yaw: rdr.read_float()?,
            pitch: rdr.read_float()?,
            on_ground: rdr.read_bool()?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum OutgoingPlayPacket {
    BlockEntityData(BlockEntity),
    CustomPayload(PlayCustomPayload),
    KeepAlive(u64),
    ChunkData {
        chunk: Chunk,
    },
    UpdateLight {
        chunk_x: i32,
        chunk_z: i32,
    },
    JoinGame {
        entity_id: i32,
        data: JoinGameData,
    },
    PlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        flags: u8,
        teleport_id: i32,
        dismount: bool,
    },
    UpdateViewPosition {
        chunk_x: i32,
        chunk_z: i32,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct JoinGameData {
    is_hardcore: bool,
    gamemode: Gamemode,
    previous_gamemode: Gamemode,
    dimension_names: Vec<String>,
    dimension_codec: DimensionCodec,
    dimension: DimensionType,
    dimension_name: String,
    hashed_seed: i64,
    max_players: i32,
    view_distance: i32,
    simulation_distance: i32,
    reduced_debug_info: bool,
    enable_respawn_screen: bool,
    is_debug: bool,
    is_flat: bool,
}

impl OutgoingPlayPacket {
    pub fn write(&self) -> Result<PacketPayload> {
        let mut payload = PacketPayload::new(self.packet_id());
        match self {
            OutgoingPlayPacket::BlockEntityData(block_entity) => {
                payload.write_position(block_entity.x, block_entity.y, block_entity.z)?;
                payload.write_var_int(block_ids::get_block_entity_id(&block_entity.id).expect("invalid block entity ID"))?;
                payload.write_nbt(&block_entity.data)?;
            }
            OutgoingPlayPacket::CustomPayload(payload_data) => {
                payload.write_string(payload_data.channel_id(), 32767)?;
                payload_data.write(&mut payload)?;
            }
            OutgoingPlayPacket::KeepAlive(v) => {
                payload.write_ulong(*v)?;
            }
            OutgoingPlayPacket::ChunkData { chunk } => {
                let mut heightmap = vec![0x0100804020100804; 36];
                heightmap.push(0x0000000020100804);
                payload.write_int(chunk.x)?;
                payload.write_int(chunk.z)?;
                payload.write_nbt(&Heightmaps {
                    motion_blocking: heightmap,
                })?;
                let mut data = Vec::<u8>::new();
                chunk.write(&mut data)?;
                payload.write_var_int(data.len() as i32)?;
                payload.write_bytes(&data)?;
                payload.write_var_int(0)?; // block entities
                // TODO: true or false? does it matter?
                payload.write_bool(true)?; // trust edges
                payload.write_ulong_array(&[])?; // sky light mask
                payload.write_ulong_array(&[])?; // block light mask
                payload.write_ulong_array(&[])?; // empty sky light mask
                payload.write_ulong_array(&[])?; // empty block light mask
                payload.write_var_int(0)?; // sky light array count
                payload.write_var_int(0)?; // block light array count
            }
            OutgoingPlayPacket::UpdateLight { chunk_x, chunk_z } => {
                payload.write_var_int(*chunk_x)?;
                payload.write_var_int(*chunk_z)?;
                payload.write_bool(true)?;
                payload.write_ulong_array(&[])?; // sky light mask
                payload.write_ulong_array(&[])?; // block light mask
                payload.write_ulong_array(&[])?; // empty sky light mask
                payload.write_ulong_array(&[])?; // empty block light mask
                payload.write_var_int(0)?; // sky light array count
                payload.write_var_int(0)?; // block light array count
            }
            OutgoingPlayPacket::JoinGame {
                entity_id,
                data:
                    JoinGameData {
                        is_hardcore,
                        gamemode,
                        previous_gamemode,
                        dimension_names,
                        dimension_codec,
                        dimension,
                        dimension_name,
                        hashed_seed,
                        max_players,
                        view_distance,
                        simulation_distance,
                        reduced_debug_info,
                        enable_respawn_screen,
                        is_debug,
                        is_flat,
                    },
            } => {
                payload.write_int(*entity_id)?;
                payload.write_bool(*is_hardcore)?;
                gamemode.write(&mut payload)?;
                previous_gamemode.write(&mut payload)?;
                payload.write_string_arr(dimension_names)?;
                payload.write_nbt(dimension_codec)?;
                payload.write_nbt(dimension)?;
                payload.write_string(&dimension_name, 32767)?;
                payload.write_long(*hashed_seed)?;
                payload.write_var_int(*max_players)?;
                payload.write_var_int(*view_distance)?;
                payload.write_var_int(*simulation_distance)?;
                payload.write_bool(*reduced_debug_info)?;
                payload.write_bool(*enable_respawn_screen)?;
                payload.write_bool(*is_debug)?;
                payload.write_bool(*is_flat)?;
            }
            OutgoingPlayPacket::PlayerPositionAndLook {
                x,
                y,
                z,
                yaw,
                pitch,
                flags,
                teleport_id,
                dismount,
            } => {
                payload.write_double(*x)?;
                payload.write_double(*y)?;
                payload.write_double(*z)?;
                payload.write_float(*yaw)?;
                payload.write_float(*pitch)?;
                payload.write_ubyte(*flags)?;
                payload.write_var_int(*teleport_id)?;
                payload.write_bool(*dismount)?;
            }
            OutgoingPlayPacket::UpdateViewPosition { chunk_x, chunk_z } => {
                payload.write_var_int(*chunk_x)?;
                payload.write_var_int(*chunk_z)?;
            }
        }
        Ok(payload)
    }

    fn packet_id(&self) -> i32 {
        match self {
            OutgoingPlayPacket::BlockEntityData(_) => 0x0a,
            OutgoingPlayPacket::CustomPayload(_) => 0x18,
            OutgoingPlayPacket::KeepAlive(_) => 0x21,
            OutgoingPlayPacket::ChunkData { .. } => 0x22,
            OutgoingPlayPacket::UpdateLight { .. } => 0x25,
            OutgoingPlayPacket::JoinGame { .. } => 0x26,
            OutgoingPlayPacket::PlayerPositionAndLook { .. } => 0x38,
            OutgoingPlayPacket::UpdateViewPosition { .. } => 0x49,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PlayCustomPayload {
    MinecraftBrand { brand: String },
}

impl PlayCustomPayload {
    fn write<W: PacketWriter>(&self, wr: &mut W) -> Result<()> {
        match self {
            PlayCustomPayload::MinecraftBrand { brand } => {
                wr.write_string(brand, 32767)?;
            }
        }
        Ok(())
    }

    fn read<R: PacketReader>(rdr: &mut R) -> Result<Option<Self>> {
        let channel_id = rdr.read_string(32767)?;
        match &*channel_id {
            "minecraft:brand" => Ok(Some(Self::read_brand(rdr)?)),
            c => {
                info!("unknown channel: {}", c);
                Ok(None)
            }
        }
    }

    fn read_brand<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::MinecraftBrand {
            brand: rdr.read_string(32767)?,
        })
    }

    fn channel_id(&self) -> &'static str {
        match self {
            PlayCustomPayload::MinecraftBrand { .. } => "minecraft:brand",
        }
    }
}

async fn send_play_packet<W: Sink<PacketPayload, Error = ProtocolError> + Unpin>(
    wr: &mut W,
    packet: OutgoingPlayPacket,
) -> Result<()> {
    let payload = packet.write()?;
    wr.send(payload).await?;
    Ok(())
}

pub async fn handle<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(
    rdr: &mut R,
    wr: &mut W,
    uuid: Uuid,
    store: ServerStore,
) -> Result<()> {
    let entity_id = store.get_player_id(uuid).await;

    send_play_packet(
        wr,
        OutgoingPlayPacket::JoinGame {
            entity_id,
            data: store.get_config().join_game_data.clone(),
        },
    )
    .await?;

    send_play_packet(
        wr,
        OutgoingPlayPacket::CustomPayload(PlayCustomPayload::MinecraftBrand {
            brand: store.get_config().server_brand.clone(),
        }),
    )
    .await?;

    let config = store.get_config();

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let position_and_look = OutgoingPlayPacket::PlayerPositionAndLook {
        x: config.spawn_point.0,
        y: config.spawn_point.1,
        z: config.spawn_point.2,
        yaw: 0.0,
        pitch: 0.0,
        flags: 0,
        teleport_id: 0,
        dismount: false,
    };

    send_play_packet(wr, position_and_look.clone()).await?;

    for chunk in store.get_chunks() {
        send_play_packet(wr, OutgoingPlayPacket::ChunkData {
            chunk: chunk.clone(),
        }).await?;

        for block_entity in store.get_block_entities() {
            send_play_packet(wr, OutgoingPlayPacket::BlockEntityData(block_entity.clone())).await?;
        }
    }

    send_play_packet(wr, OutgoingPlayPacket::UpdateViewPosition {
        chunk_x: 0,
        chunk_z: 0,
    }).await?;

    send_play_packet(wr, position_and_look.clone()).await?;

    let mut keep_alive_interval = interval(Duration::from_millis(1000));
    keep_alive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            d = rdr.try_next() => {
                match d {
                    Ok(Some(mut packet_data)) => {
                        let packet = IncomingPlayPacket::read(packet_data.packet_id, &mut packet_data)?;
                        if let Some(packet) = packet {
                            match &packet {
                                IncomingPlayPacket::TeleportConfirm { .. }
                                | IncomingPlayPacket::ClientSettings { .. }
                                | IncomingPlayPacket::CustomPayload(_) => info!("got packet: {:?}", packet),
                                _ => {}
                            }
                        } else {
                            // only log these at a high level when compiled in debug mode
                            #[cfg(debug_assertions)]
                            warn!(
                                "unknown play packet id {}, ignoring ({:#02x})",
                                packet_data.packet_id, packet_data
                            );

                            #[cfg(not(debug_assertions))]
                            debug!(
                                "unknown play packet id {}, ignoring ({:#02x})",
                                packet_data.packet_id, packet_data
                            );
                        }
                    },
                    Err(e) => {
                        return Err(e);
                    },
                    _ => break,
                }
            }
            _ = keep_alive_interval.tick() => {
                debug!("Sending keep alive packet");
                let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("current time is before the unix epoch!?").as_secs();
                send_play_packet(wr, OutgoingPlayPacket::KeepAlive(now)).await?;
            }
        }
    }

    Ok(())
}
