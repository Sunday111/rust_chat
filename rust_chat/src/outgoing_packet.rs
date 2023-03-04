use std::fmt::Display;
use std::io::Write;

pub struct PacketInProgress {
    data: Vec<u8>,
    sent: usize,
}

const MAX_SEND_CHUNK: usize = 1024;

impl PacketInProgress {
    pub fn advance<Stream>(mut self, stream: &mut Stream) -> (Packet, usize)
    where
        Stream: Write,
    {
        assert!(self.sent < self.data.len());

        let write_result = {
            let remaining_bytes_count = self.data.len() - self.sent;
            let send_bytes_count = std::cmp::min(remaining_bytes_count, MAX_SEND_CHUNK);
            stream.write(&self.data[self.sent..self.sent + send_bytes_count])
        };

        match write_result {
            Ok(bytes_sent) => {
                self.sent += bytes_sent;

                if self.sent < self.data.len() {
                    (Packet::InProgress(self), bytes_sent)
                } else {
                    (Packet::Sent, bytes_sent)
                }
            }
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    (Packet::InProgress(self), 0)
                } else {
                    (Packet::Failed(PacketError::StreamError(error)), 0)
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum PacketError {
    ZeroSizedPacket,
    StreamError(std::io::Error),
}

impl Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroSizedPacket => write!(f, "Attempt to send zero-sized packet"),
            Self::StreamError(err) => write!(f, "stream error: {err}"),
        }
    }
}

pub enum Packet {
    InProgress(PacketInProgress),
    Sent,
    Failed(PacketError),
}

impl Packet {
    pub fn new(bytes: &[u8]) -> Packet {
        if bytes.len() == 0 {
            return Packet::Failed(PacketError::ZeroSizedPacket)
        }

        let mut data = Vec::new() as Vec<u8>;
        data.reserve(bytes.len() + 4);

        let len = bytes.len() as i32;
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(bytes);

        Packet::InProgress(PacketInProgress {
            data: data,
            sent: 0,
        })
    }

    pub fn advance<Stream>(self, stream: &mut Stream) -> Packet
    where
        Stream: Write,
    {
        match self {
            Packet::InProgress(in_progress) => in_progress.advance(stream).0,
            Packet::Failed(failed) => Packet::Failed(failed),
            Packet::Sent => Packet::Sent,
        }
    }

    pub fn advance_until_sent<Stream>(mut self, stream: &mut Stream) -> Packet
    where
        Stream: Write,
    {
        let mut finished = false;
        while !finished {
            self = match self {
                Packet::InProgress(in_progress) => in_progress.advance(stream).0,
                Packet::Sent => {
                    finished = true;
                    Packet::Sent
                }
                Packet::Failed(err) => {
                    finished = true;
                    Packet::Failed(err)
                }
            }
        }

        self
    }

    pub fn advance_until_would_block<Stream>(mut self, stream: &mut Stream) -> Packet
    where
        Stream: Write,
    {
        let mut finished = false;
        while !finished {
            self = match self {
                Packet::InProgress(in_progress) => {
                    let (packet, bytes_sent) = in_progress.advance(stream);
                    if bytes_sent == 0 {
                        finished = true;
                    }
                    packet
                }
                Packet::Sent => {
                    finished = true;
                    Packet::Sent
                }
                Packet::Failed(err) => {
                    finished = true;
                    Packet::Failed(err)
                }
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        generate_random_string, next_localhost_address,
    };
    use std::io::{BufWriter, Write, Read};

    #[test]
    fn create_new_packet() {
        let payload = "Hello, world!";
        let packet = Packet::new(payload.as_bytes());
        if let Packet::InProgress(in_progress) = packet {
            assert_eq!(in_progress.sent, 0);
        } else {
            panic!("Unexpected packet state")
        }
    }

    #[test]
    fn detailed_advance() {
        let payload = "Hello, world!";
        let mut buffer: Vec<u8> = Vec::new();

        {
            let mut writer = BufWriter::new(&mut buffer);
            let packet = Packet::new(payload.as_bytes());
            match packet.advance(&mut writer) {
                Packet::Sent => {}
                _ => {
                    panic!("Unexpected packet state")
                }
            }

            // flush stream
            writer.flush().unwrap();
        }

        assert_eq!(
            u32::from_be_bytes(buffer[0..4].try_into().unwrap()),
            payload.len() as u32
        );
        assert_eq!(&buffer[4..], payload.as_bytes());
    }

    #[test]
    fn detailed_advance_big_payload() {
        // size of backet is bigger than MAX_SEND_CHUNK so it can't be sent in one advance call
        let payload = generate_random_string(1234, MAX_SEND_CHUNK, MAX_SEND_CHUNK);
        let mut buffer: Vec<u8> = Vec::new();

        {
            let mut writer = BufWriter::new(&mut buffer);
            let packet = Packet::new(payload.as_bytes());
            let packet = match packet.advance(&mut writer) {
                Packet::InProgress(state) => {
                    assert_eq!(state.sent, MAX_SEND_CHUNK);
                    Packet::InProgress(state)
                }
                _ => {
                    panic!("Unexpected packet state")
                }
            };

            match packet.advance(&mut writer) {
                Packet::Sent => {}
                _ => {
                    panic!("Unexpected packet state")
                }
            }

            // flush stream
            writer.flush().unwrap();
        }

        assert_eq!(
            u32::from_be_bytes(buffer[0..4].try_into().unwrap()),
            payload.len() as u32
        );
        assert_eq!(&buffer[4..], payload.as_bytes());
    }

    #[test]
    fn advance_until_sent() {
        // size of backet is bigger than MAX_SEND_CHUNK so it can't be sent in one advance call
        let payload = generate_random_string(1234, MAX_SEND_CHUNK, MAX_SEND_CHUNK);

        let buffer = {
            let mut buffer: Vec<u8> = Vec::new();
            {
                let mut writer = BufWriter::new(&mut buffer);
                let packet = Packet::new(payload.as_bytes());
                match packet.advance_until_sent(&mut writer) {
                    Packet::Sent => {}
                    _ => {
                        panic!("Unexpected packet state")
                    }
                }

                // flush stream
                writer.flush().unwrap();
            }
            buffer
        };

        assert_eq!(
            u32::from_be_bytes(buffer[0..4].try_into().unwrap()),
            payload.len() as u32
        );
        assert_eq!(&buffer[4..], payload.as_bytes());
    }

    

    #[test]
    fn advance_until_sent_tcp() {
        let payload = generate_random_string(1234, MAX_SEND_CHUNK * 2, MAX_SEND_CHUNK * 10);
        let address = next_localhost_address();

        let join_handle = {
            // clone values before they move to spawned thread
            let payload = payload.clone();
            let address = address.clone();
            std::thread::spawn(move || {
                let listener = std::net::TcpListener::bind(address).unwrap();
                match listener.incoming().next().expect("") {
                    Ok(mut stream) => {
                        let mut data = Vec::new();
                        stream.read_to_end(&mut data).expect("Failed to read from stream");
                        assert_eq!(&data[4..], payload.as_bytes());
                    }
                    Err(e) => {
                        eprintln!("failed to accept client connection: {}", e);
                    }
                }
            })
        };

        {
            let mut stream = std::net::TcpStream::connect(address).unwrap();
            stream
                .set_nonblocking(true)
                .expect("Can't make stream nonblocking");
            
            let packet = Packet::new(payload.as_bytes());
            match packet.advance_until_sent(&mut stream) {
                Packet::Sent => {}
                _ => {
                    panic!("Unexpected packet state")
                }
            }
        }

        join_handle.join().unwrap()
    }
}
