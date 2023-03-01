use crate::{ChatError, ChatResult};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

pub struct PacketInProgress {
    data: Vec<u8>,
    sent: usize,
}

const MAX_SEND_CHUNK: usize = 1024;

impl PacketInProgress {
    pub fn advance<Stream>(mut self, stream: &mut Stream) -> Packet
        where Stream: Write
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
                    Packet::InProgress(self)
                } else {
                    Packet::Sent
                }
            }
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    Packet::InProgress(self)
                } else {
                    Packet::Failed(SendPacketError::StreamError(error))
                }
            }
        }
    }
}

pub enum SendPacketError {
    StreamError(std::io::Error),
}

pub enum Packet {
    InProgress(PacketInProgress),
    Sent,
    Failed(SendPacketError),
}

impl Packet {
    pub fn new(bytes: &[u8]) -> ChatResult<Packet> {
        if bytes.len() == 0 {
            return Err(ChatError("Trying to send an empty packet".to_string()));
        }

        let mut data = Vec::new() as Vec<u8>;
        data.reserve(bytes.len() + 4);

        let len = bytes.len() as i32;
        data.extend_from_slice(&len.to_be_bytes());
        data.extend_from_slice(bytes);

        Ok(Packet::InProgress(PacketInProgress {
            data: data,
            sent: 0
        }))
    }

    pub fn advance<Stream>(self, stream: &mut Stream) -> Packet
        where Stream: Write
    {
        match self {
            Packet::InProgress(in_progress) => in_progress.advance(stream),
            Packet::Failed(failed) => Packet::Failed(failed),
            Packet::Sent => Packet::Sent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufWriter, Write};

    #[test]
    fn create_new_packet() {
        let payload = "Hello, world!";
        let packet = Packet::new(payload.as_bytes()).unwrap();
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
            let packet = Packet::new(payload.as_bytes()).unwrap();
            match packet.advance(&mut writer) {
                Packet::Sent => {},
                _ => { panic!("Unexpected packet state") }
            }

            // flush stream
            writer.flush().unwrap();
        }

        assert_eq!(u32::from_be_bytes(buffer[0..4].try_into().unwrap()), payload.len() as u32);
        assert_eq!(&buffer[4..], payload.as_bytes());
    }

    #[test]
    fn detailed_advance_big_payload() {
        // size of backet is bigger than MAX_SEND_CHUNK so it can't be sent in one advance call
        let payload = "Hello, world!".repeat((MAX_SEND_CHUNK / 13) + 1);
        let mut buffer: Vec<u8> = Vec::new();

        {
            let mut writer = BufWriter::new(&mut buffer);
            let packet = Packet::new(payload.as_bytes()).unwrap();
            let packet = match packet.advance(&mut writer) {
                Packet::InProgress(state) => {
                    assert_eq!(state.sent, MAX_SEND_CHUNK);
                    Packet::InProgress(state)
                },
                _ => { panic!("Unexpected packet state") }
            };

            match packet.advance(&mut writer) {
                Packet::Sent => {},
                _ => { panic!("Unexpected packet state") }
            }

            // flush stream
            writer.flush().unwrap();
        }

        assert_eq!(u32::from_be_bytes(buffer[0..4].try_into().unwrap()), payload.len() as u32);
        assert_eq!(&buffer[4..], payload.as_bytes());
    }
}
