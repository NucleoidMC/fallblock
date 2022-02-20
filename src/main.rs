use std::net::SocketAddr;

use futures::Sink;
use futures::TryStream;
use futures::TryStreamExt;
use protocol::MinecraftFramedCodec;
use protocol::PacketData;
use protocol::PacketPayload;
use protocol::ProtocolState;
use protocol::handshake::HandshakePacket;
use tokio::net::TcpStream;

use tokio::net::TcpListener;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;
use util::ProtocolError;
use crate::constants::PROTOCOL_VERSION;
use crate::store::ServerStore;
use crate::util::Result;
use crate::world::map_template;

pub mod protocol;
pub mod io;
pub mod util;
pub mod constants;
pub mod store;
pub mod world;
pub mod config;

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Hello, world!");

    let config = config::load_config();

    info!("Loading chunks...");
    let map_template = map_template::load_template(&config.map_file)
        .expect("failed to read map file");
    info!("World ready");

    let store = ServerStore::new(config, map_template);

    let listener = TcpListener::bind("127.0.0.1:25566").await?;
    info!("Listening on {}", listener.local_addr()?);
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(peer_addr, stream, store).await {
                error!("failed to handle connection from {}: {}", peer_addr, e);
            }
        });
    }
}

#[instrument(skip(stream, store))]
async fn handle_connection(peer_addr: SocketAddr, stream: TcpStream, store: ServerStore) -> Result<()> {
    info!("handling connection from {}", peer_addr);

    let (rd, wr) = tokio::io::split(stream);
    let mut framed_read = FramedRead::new(rd, MinecraftFramedCodec);
    let mut framed_write = FramedWrite::new(wr, MinecraftFramedCodec);

    let handshake = handshake(&mut framed_read).await?;

    if let Some(handshake) = handshake {
        info!("got handshake packet: {:?}", handshake);
        if handshake.protocol_version != PROTOCOL_VERSION {
            warn!("unsupported protocol version: {}", handshake.protocol_version);
        } else {
            handle_next_phase(&mut framed_read, &mut framed_write, handshake.next_state, store).await?;
            info!("Connection handling complete!");
        }
    }

    Ok(())
}

async fn handle_next_phase<
    R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin,
    W: Sink<PacketPayload, Error = ProtocolError> + Unpin,
>(rdr: &mut R, wr: &mut W, next_state: ProtocolState, store: ServerStore) -> Result<()> {
    match next_state {
        ProtocolState::Login => protocol::login::handle(rdr, wr, store).await,
        ProtocolState::Status => protocol::status::handle(rdr, wr, store).await,
    }
}

async fn handshake<R: TryStream<Ok = PacketData, Error = ProtocolError> + Unpin>(rdr: &mut R) -> Result<Option<HandshakePacket>> {
    if let Some(mut packet) = rdr.try_next().await? {
        if packet.packet_id != 0 {
            Err(ProtocolError::MissingHandshake)
        } else {
            HandshakePacket::read(&mut packet).map(Option::Some)
        }
    } else {
        Ok(None)
    }
}
