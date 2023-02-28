use crate::{ChatError, ChatResult};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

pub struct PacketInProgress {
    data: Vec<u8>,
    sent: usize,
    stream: Rc<RefCell<dyn Write>>,
}

const MAX_SEND_CHUNK: usize = 1024;

impl PacketInProgress {
    pub fn advance(mut self) -> Packet {
        assert!(self.sent < self.data.len());

        let write_result = {
            let mut stream = self.stream.borrow_mut();
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
    pub fn new(bytes: &[u8], stream: Rc<RefCell<Box<dyn Write>>>) -> ChatResult<Packet> {
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
            sent: 0,
            stream: stream,
        }))
    }

    pub fn advance(self) -> Packet {
        match self {
            Packet::InProgress(in_progress) => in_progress.advance(),
            Packet::Failed(failed) => Packet::Failed(failed),
            Packet::Sent => Packet::Sent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand;
    use rand::{Rng, SeedableRng};
    use std::io::{BufWriter, Write, Read};

    #[test]
    fn create_new_packet() {
        let payload = "Hello, world!";
        let writer_box = Box::new(BufWriter::new(Vec::new() as Vec<u8>)) as Box<dyn Write>;
        let stream = Rc::new(RefCell::new(writer_box));
        let packet = Packet::new(payload.as_bytes(), Rc::clone(&stream)).unwrap();
        if let Packet::InProgress(in_progress) = packet {
            assert_eq!(in_progress.sent, 0);
        } else {
            panic!("Unexpected packet state")
        }
    }

    struct SharedBuffer {
        data: Rc<RefCell<Vec<u8>>>
    }

    impl SharedBuffer {
        pub fn new() -> SharedBuffer {
            SharedBuffer { data: Rc::new(RefCell::new(Vec::new())) }
        }
    }

    impl Write for SharedBuffer {
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }

        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut data = self.data.borrow_mut();
            data.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    impl Clone for SharedBuffer {
        fn clone(&self) -> Self {
            SharedBuffer { data: Rc::clone(&self.data) }
        }
    }

    #[test]
    fn detailed_advance() {
        let payload = "Hello, world!";
        let shared_buffer = SharedBuffer::new();

        {
            let writer_box = Box::new(BufWriter::new(shared_buffer.clone())) as Box<dyn Write>;
            let stream = Rc::new(RefCell::new(writer_box));
            let packet = Packet::new(payload.as_bytes(), Rc::clone(&stream)).unwrap();
            match packet.advance() {
                Packet::Sent => {},
                _ => { panic!("Unexpected packet state") }
            }

            // flush stream
            stream.borrow_mut().flush().unwrap();
        }

        let buffer = shared_buffer.data.borrow_mut();
        assert_eq!(u32::from_be_bytes(buffer[0..4].try_into().unwrap()), payload.len() as u32);
        assert_eq!(&buffer[4..], payload.as_bytes());
    }

    #[test]
    fn detailed_advance_big_payload() {
        // size of backet is bigger than MAX_SEND_CHUNK so it can't be sent in one advance call
        let payload = "Hello, world!".repeat((MAX_SEND_CHUNK / 13) + 1);
        let shared_buffer = SharedBuffer::new();

        {
            let writer_box = Box::new(BufWriter::new(shared_buffer.clone())) as Box<dyn Write>;
            let stream = Rc::new(RefCell::new(writer_box));
            let packet = Packet::new(payload.as_bytes(), Rc::clone(&stream)).unwrap();
            let packet = match packet.advance() {
                Packet::InProgress(state) => {
                    assert_eq!(state.sent, MAX_SEND_CHUNK);
                    Packet::InProgress(state)
                },
                _ => { panic!("Unexpected packet state") }
            };

            match packet.advance() {
                Packet::Sent => {},
                _ => { panic!("Unexpected packet state") }
            }

            // flush stream
            stream.borrow_mut().flush().unwrap();
        }

        let buffer = shared_buffer.data.borrow_mut();
        assert_eq!(u32::from_be_bytes(buffer[0..4].try_into().unwrap()), payload.len() as u32);
        assert_eq!(&buffer[4..], payload.as_bytes());
    }
}
