use std::io::Cursor;

use futures::{TryStream, TryStreamExt, Sink, SinkExt};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::{io::{PacketReader, PacketWriter}, util::{Result, ProtocolError, self}, store::ServerStore, protocol::play};

use super::{PacketData, PacketPayload};

pub enum IncomingLoginPacket {
    LoginStart {
        username: String,
    },
    LoginPluginResponse {
        message_id: i32,
        successful: bool,
        data: Vec<u8>,
    },
}

impl IncomingLoginPacket {
    pub fn read<R: PacketReader>(packet_id: i32, rdr: &mut R) -> Result<Self> {
        match packet_id {
            0 => Self::read_login_start(rdr),
            2 => Self::read_login_plugin_response(rdr),
            v => {
                debug!(%v, "invalid packet id");
                Err(ProtocolError::InvalidPacketId(v))
            },
        }
    }

    fn read_login_start<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::LoginStart {
            username: rdr.read_string(16)?,
        })
    }

    fn read_login_plugin_response<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        let message_id = rdr.read_var_int()?;
        let successful = rdr.read_bool()?;
        let data = if successful {
            rdr.read_remaining()?
        } else {
            vec![]
        };
        Ok(Self::LoginPluginResponse {
            message_id,
            successful,
            data,
        })
    }
}

pub enum OutgoingLoginPacket {
    LoginSuccess {
        uuid: Uuid,
        username: String,
    },
    LoginPluginRequest {
        message_id: i32,
        channel: String,
        data: Vec<u8>,
    },
}

impl OutgoingLoginPacket {
    pub fn write(&self) -> Result<PacketPayload> {
        let mut payload = PacketPayload::new(self.packet_id());
        match self {
            OutgoingLoginPacket::LoginSuccess { uuid, username } => {
                payload.write_uuid(uuid)?;
                payload.write_string(&username, 16)?;
            },
            OutgoingLoginPacket::LoginPluginRequest { message_id, channel, data } => {
                payload.write_var_int(*message_id)?;
                payload.write_string(&channel, 32767)?;
                payload.write_bytes(&*data)?;
            },
        }
        Ok(payload)
    }

    fn packet_id(&self) -> i32 {
        match self {
            OutgoingLoginPacket::LoginSuccess { .. } => 0x02,
            OutgoingLoginPacket::LoginPluginRequest { .. } => 0x04,
        }
    }
}

pub async fn handle<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(rdr: &mut R, wr: &mut W, store: ServerStore) -> Result<()> {
    if let Some(mut packet) = rdr.try_next().await? {
        if let IncomingLoginPacket::LoginStart { username } = IncomingLoginPacket::read(packet.packet_id, &mut packet)? {
            return if store.get_config().modern_forwarding_key.is_some() {
                modern_forwarding_handshake(rdr, wr, store, username).await
            } else {
                let uuid = util::offline_mode_uuid(&username);
                complete_login(rdr, wr, store, uuid, username).await
            };
        }
    }

    Ok(())
}

async fn modern_forwarding_handshake<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(rdr: &mut R, wr: &mut W, store: ServerStore, username: String) -> Result<()> {
    debug!("Performing modern forwarding handshake with user: {}", username);
    wr.send(OutgoingLoginPacket::LoginPluginRequest {
        message_id: 0x01,
        channel: "velocity:player_info".into(),
        data: vec![],
    }.write()?).await?;
    if let Some(mut packet) = rdr.try_next().await? {
        if let IncomingLoginPacket::LoginPluginResponse { message_id, successful, data } = IncomingLoginPacket::read(packet.packet_id, &mut packet)? {
            if !successful {
                warn!(?packet, "failed to perform modern player forwarding: not supported by client");
                return Ok(());
            }
            if message_id == 0x01 {
                // we got a response!
                debug!(?data, "got a forwarding data data data");
                let (sig, payload) = (&data[..32], &data[32..]);
                let modern_forwarding_key = store
                    .get_config()
                    .modern_forwarding_key
                    .as_ref()
                    .expect("called modern_forwarding_handshake when modern forwarding is disabled");
                if !check_signature(modern_forwarding_key.as_bytes(), sig, payload) {
                    warn!(%username, ?packet, "modern forwarding information has invalid signature");
                } else {
                    let mut payload = Cursor::new(payload);
                    let forwarding_version = payload.read_var_int()?;
                    let client_address = payload.read_string(32767)?;
                    let uuid = payload.read_uuid()?;
                    let username = payload.read_string(16)?;
                    debug!(%forwarding_version, %client_address, %uuid, %username, "completed modern information handshake");
                    complete_login(rdr, wr, store, uuid, username).await?;
                }
            } else {
                warn!(?packet, "got unknown plugin response");
            }
        }
    }

    Ok(())
}

async fn complete_login<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(rdr: &mut R, wr: &mut W, store: ServerStore, uuid: Uuid, username: String)  -> Result<()> {
    info!(%username, %uuid, "completing login");
    let success_packet = OutgoingLoginPacket::LoginSuccess {
        uuid,
        username,
    }.write()?;
    wr.send(success_packet).await?;
    play::handle(rdr, wr, uuid, store).await
}

fn check_signature(key: &[u8], sig: &[u8], payload: &[u8]) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .expect("HMAC can take a key of any size");
    mac.update(payload);
    let result = mac.finalize().into_bytes();
    &result[..] == sig
}
