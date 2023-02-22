use std::net::SocketAddr;
use std::net::TcpStream;
use std::io::Read;

/* Just created connection.
 * Message with handshake data has not been accepted yet
 */
struct HandshakeState {
    data: Vec<u8>,
}

impl HandshakeState {
    fn new() -> HandshakeState {
        HandshakeState {
            data: vec![0;1024]
        }
    }

    fn read_data(&mut self, stream: &mut TcpStream) {
        //let r = stream.read()
        // let reader = BufReader::new(stream);
    }
}

/* Initialized and accepted connection
 */
struct EstablishedState {}

impl EstablishedState {
    fn new() -> EstablishedState {
        EstablishedState {
        }
    }

    fn read_data(&mut self, stream: &mut TcpStream) {
        //let r = stream.read()
        // let reader = BufReader::new(stream);
    }
}

/* Closed connection
 */
struct ClosedState {}

enum ConnectionState {
    HandShake(HandshakeState),
    Established(EstablishedState),
    Closed(ClosedState),
}

pub struct Connection {
    stream: TcpStream,
    address: SocketAddr,
    state: ConnectionState,
}

impl Connection {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Connection {
        Connection {
            stream: stream,
            address: addr,
            state: ConnectionState::HandShake(HandshakeState::new()),
        }
    }

    pub fn read_data(&mut self) {
        match &mut self.state {
            ConnectionState::HandShake(s) => {
                s.read_data(&mut self.stream);

            },
            ConnectionState::Established(s) => {
                s.read_data(&mut self.stream);
            }
            ConnectionState::Closed(_) => {}
        }
    }
}
