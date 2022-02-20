use futures::{TryStream, Sink, SinkExt, TryStreamExt};
use mc_chat::ChatComponent;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::{util::{ProtocolError, Result}, io::{PacketReader, PacketWriter}, store::ServerStore, constants::ProtocolVersion};

use super::{PacketData, PacketPayload};

enum IncomingStatusPacket {
    Request,
    Ping(i64),
}

impl IncomingStatusPacket {
    pub fn read<R: PacketReader>(packet_id: i32, rdr: &mut R) -> Result<Option<Self>> {
        match packet_id {
            0x00 => Ok(Some(Self::Request)),
            0x01 => Ok(Some(Self::Ping(rdr.read_long()?))),
            _ => Ok(None),
        }
    }
}

enum OutgoingStatusPacket {
    Response(ServerListPingResponse),
    Pong(i64),
}

impl OutgoingStatusPacket {
    pub fn write(&self) -> Result<PacketPayload> {
        let mut payload = PacketPayload::new(self.packet_id());
        match self {
            Self::Response(response) => {
                payload.write_json(response)?;
            }
            Self::Pong(v) => {
                payload.write_long(*v)?;
            }
        }
        Ok(payload)
    }

    fn packet_id(&self) -> i32 {
        match self {
            Self::Response(_) => 0x00,
            Self::Pong(_) => 0x01,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerListPingResponse {
    version: ProtocolVersion,
    players: ServerListPlayers,
    description: ChatComponent,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerListPlayers {
    max: u32,
    online: u32,
    sample: Vec<SamplePlayer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SamplePlayer {
    name: String,
    id: Uuid,
}

async fn send_status_packet<W: Sink<PacketPayload, Error = ProtocolError> + Unpin>(
    wr: &mut W,
    packet: OutgoingStatusPacket,
) -> Result<()> {
    let payload = packet.write()?;
    wr.send(payload).await?;
    Ok(())
}

async fn recv_status_packet<R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin>(
    rdr: &mut R,
) -> Result<IncomingStatusPacket> {
    let mut data = rdr.try_next().await?.ok_or(ProtocolError::NoPacket)?;
    IncomingStatusPacket::read(data.packet_id, &mut data.data)?.ok_or(ProtocolError::InvalidPacketId(data.packet_id))
}

pub async fn handle<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(
    rdr: &mut R,
    wr: &mut W,
    store: ServerStore,
) -> Result<()> {
    if let IncomingStatusPacket::Request = recv_status_packet(rdr).await? {
        send_status_packet(wr, OutgoingStatusPacket::Response(store.get_config().status.clone())).await?;
    } else {
        return Err(ProtocolError::MissingRequest);
    }

    if let IncomingStatusPacket::Ping(v) = recv_status_packet(rdr).await? {
        send_status_packet(wr, OutgoingStatusPacket::Pong(v)).await?;
    }

    Ok(())
}
