use futures::{TryStream, TryStreamExt, Sink, SinkExt};
use uuid::Uuid;

use crate::{io::{PacketReader, PacketWriter}, util::{Result, ProtocolError, self}, store::ServerStore, protocol::play};

use super::{PacketData, PacketPayload};

pub enum IncomingLoginPacket {
    LoginStart {
        username: String,
    },
    LoginPluginResponse {
        // TODO: Do we need this?
    }
}

impl IncomingLoginPacket {
    pub fn read<R: PacketReader>(packet_id: i32, rdr: &mut R) -> Result<Self> {
        match packet_id {
            0 => Self::read_login_start(rdr),
            2 => todo!(),
            v => Err(ProtocolError::InvalidPacketId(v)),
        }
    }

    fn read_login_start<R: PacketReader>(rdr: &mut R) -> Result<Self> {
        Ok(Self::LoginStart {
            username: rdr.read_string(16)?,
        })
    }
}

pub enum OutgoingLoginPacket {
    LoginSuccess {
        uuid: Uuid,
        username: String,
    }
    // TODO: We probably need the plugin channel stuff here for velocity integration.
}

impl OutgoingLoginPacket {
    pub fn write(&self) -> Result<PacketPayload> {
        let mut payload = PacketPayload::new(self.packet_id());
        match self {
            OutgoingLoginPacket::LoginSuccess { uuid, username } => {
                payload.write_uuid(uuid)?;
                payload.write_string(&username, 16)?;
            },
        }
        Ok(payload)
    }

    fn packet_id(&self) -> i32 {
        match self {
            OutgoingLoginPacket::LoginSuccess { .. } => 0x02,
        }
    }
}

pub async fn handle<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(rdr: &mut R, wr: &mut W, store: ServerStore) -> Result<()> {
    if let Some(mut packet) = rdr.try_next().await? {
        if let IncomingLoginPacket::LoginStart { username } = IncomingLoginPacket::read(packet.packet_id, &mut packet)? {
            let uuid = util::offline_mode_uuid(&username);
            info!("logging in as user: {} (uuid={})", username, uuid);
            let success_packet = OutgoingLoginPacket::LoginSuccess {
                uuid,
                username,
            }.write()?;
            wr.send(success_packet).await?;
            play::handle(rdr, wr, uuid, store).await?;
            // TODO: Initiate play phase and handle full connection
        }
    }

    Ok(())
}
