use crate::packet_receiver::PacketReceiver;
use crate::packet_sender::PacketSender;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::net::TcpStream;

type IncomingPacket = crate::incoming_packet::Packet;
type IncomingPacketError = crate::incoming_packet::PacketError;

type OutgoingPacket = crate::outgoing_packet::Packet;
type OutgoingPacketError = crate::outgoing_packet::PacketError;

pub struct ConnectionInfo {
    pub address: std::net::SocketAddr,
}

pub struct LoginInfo {
    pub user: String,
}

/* Just created connection.
 * Message with handshake data has not been accepted yet
 */
pub struct HandshakeState {
    packet: IncomingPacket,
    stream: TcpStream,
}

#[derive(Serialize, Deserialize)]
struct HandshakeMessage {
    username: String,
}

impl HandshakeState {
    fn new(stream: TcpStream) -> HandshakeState {
        stream
            .set_nonblocking(true)
            .expect("Failed to make tcp stream non-blocking");
        HandshakeState {
            packet: IncomingPacket::new(),
            stream: stream,
        }
    }

    fn receive(mut self) -> Connection {
        self.packet = self.packet.advance_until_would_block(&mut self.stream);
        match self.packet {
            IncomingPacket::Received(data) => {
                match serde_json::from_slice::<HandshakeMessage>(&data) {
                    Ok(message) => {
                        println!("New connection with username: {}", message.username);
                        Connection::Established(EstablishedConnection {
                            connection_info: ConnectionInfo {
                                address: self.stream.peer_addr().expect("Failed to get peer address")
                            },
                            login_info: LoginInfo {
                                user: message.username,
                            },
                            stream: self.stream,
                            sender: PacketSender::new(),
                            receiver: PacketReceiver::new(),
                        })
                    }
                    Err(parse_err) => {
                        println!("failed to parse packet as handshake message: {parse_err:#?}");
                        Connection::Closed(ClosedConnection {
                            reason: ConnectionClosedReason::InvalidHandshakeMessage,
                        })
                    }
                }
            }
            IncomingPacket::Failed(err) => Connection::Closed(ClosedConnection {
                reason: ConnectionClosedReason::PacketReceiveError(err),
            }),
            IncomingPacket::InProgress(state) => Connection::HandShake(HandshakeState {
                packet: IncomingPacket::InProgress(state),
                stream: self.stream,
            }),
            IncomingPacket::Size(state) => Connection::HandShake(HandshakeState {
                packet: IncomingPacket::Size(state),
                stream: self.stream,
            }),
        }
    }

    fn send(self) -> Connection {
        // does nothing for now
        Connection::HandShake(self)
    }
}

/* Initialized and accepted connection
 */
pub struct EstablishedConnection {
    connection_info: ConnectionInfo,
    login_info: LoginInfo,
    stream: TcpStream,

    sender: PacketSender,
    receiver: PacketReceiver,
}

#[derive(Serialize, Deserialize)]
pub enum CommandType {
    MessageFromUser,
}

#[derive(Serialize, Deserialize)]
pub struct MesasgeFromUser {
    pub username: String,
    pub text: String,
}

impl EstablishedConnection {
    pub fn receive(mut self) -> Connection {
        match self.receiver.advance(&mut self.stream) {
            Ok(()) => Connection::Established(self),
            Err(err) => Connection::Closed(ClosedConnection {
                reason: ConnectionClosedReason::PacketReceiveError(err),
            }),
        }
    }

    pub fn send(mut self) -> Connection {
        match self.sender.advance(&mut self.stream) {
            Ok(()) => Connection::Established(self),
            Err(err) => Connection::Closed(ClosedConnection {
                reason: ConnectionClosedReason::PacketSendError(err),
            }),
        }
    }

    pub fn take_message(&mut self) -> Option<Vec<u8>> {
        self.receiver.pop_packet()
    }

    pub fn enqueue_message(&mut self, message: String) {
        self.sender.add_to_send_queue(message.into_bytes());
    }
}

enum ConnectionClosedReason {
    InvalidHandshakeMessage,
    PacketSendError(OutgoingPacketError),
    PacketReceiveError(IncomingPacketError),
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

    pub fn receive(self) -> Connection {
        match self {
            Connection::HandShake(state) => state.receive(),
            Connection::Established(state) => state.receive(),
            Connection::Closed(state) => Connection::Closed(state),
        }
    }

    pub fn send(self) -> Connection {
        match self {
            Connection::HandShake(state) => state.send(),
            Connection::Established(state) => state.send(),
            Connection::Closed(state) => Connection::Closed(state),
        }
    }
}
