use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::net::TcpStream;

use crate::packet::{Packet, PacketError};

/* Just created connection.
 * Message with handshake data has not been accepted yet
 */
pub struct HandshakeState {
    packet: Packet,
    stream: TcpStream,
}

#[derive(Serialize, Deserialize)]
struct HandshakeMessage {
    username: String,
}

impl HandshakeState {
    fn new(stream: TcpStream) -> HandshakeState {
        HandshakeState {
            packet: Packet::new(),
            stream: stream,
        }
    }

    fn read(mut self) -> Connection {
        self.packet = self.packet.advance_until_would_block(&mut self.stream);
        match self.packet {
            Packet::Complete(data) => match serde_json::from_slice::<HandshakeMessage>(&data) {
                Ok(message) => Connection::Established(EstablishedConnection {
                    info: ConnectionInfo { username: message.username },
                    stream: self.stream,
                }),
                Err(_) => Connection::Closed(ClosedConnection {
                    reason: ConnectionClosedReason::InvalidHandshakeMessage,
                }),
            },
            Packet::Failed(err) => Connection::Closed(ClosedConnection {
                reason: ConnectionClosedReason::PacketReadingError(err),
            }),
            Packet::InProgress(state) => Connection::HandShake(HandshakeState {
                packet: Packet::InProgress(state),
                stream: self.stream,
            }),
            Packet::Size(state) => Connection::HandShake(HandshakeState {
                packet: Packet::Size(state),
                stream: self.stream,
            }),
        }
    }
}

/* Initialized and accepted connection
 */
struct ConnectionInfo {
    username: String,
}
pub struct EstablishedConnection {
    info: ConnectionInfo,
    stream: TcpStream,
}

impl EstablishedConnection {
    fn read_data(&mut self, stream: &mut TcpStream) {
        //let r = stream.read()
        // let reader = BufReader::new(stream);
    }
}

enum ConnectionClosedReason {
    InvalidHandshakeMessage,
    PacketReadingError(PacketError),
    StreamError,
}

/* Closed connection
 */
pub struct ClosedConnection {
    reason: ConnectionClosedReason,
}

pub enum Connection {
    HandShake(HandshakeState),
    Established(EstablishedConnection),
    Closed(ClosedConnection),
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection::HandShake(HandshakeState::new(stream))
    }

    pub fn read_once(self) -> Connection {
        match self {
            Connection::HandShake(state) => state.read(),
            Connection::Established(state) => Connection::Established(state),
            Connection::Closed(state) => Connection::Closed(state),
        }
    }
}
