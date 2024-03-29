use std::io::Read;
use std::fmt::Display;

const MAX_PACKET_SIZE: u32 = 65536;

pub struct PacketReadingSize {
    size: u32,
    read: usize,
}

impl PacketReadingSize {
    fn advance<T>(mut self, stream: &mut T) -> (Packet, usize)
    where
        T: Read,
    {
        assert!(self.read < 4);
        let remaining_buf = unsafe {
            let pointer = (&mut self.size as *mut _ as *mut u8).add(self.read);
            std::slice::from_raw_parts_mut(pointer, 4 - self.read)
        };
        match stream.read(remaining_buf) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    println!("zero bytes");
                    return (Packet::Failed(PacketError::StreamClosed), 0);
                }

                assert!(self.read + bytes_read <= 4);
                self.read += bytes_read;

                if self.read == 4 {
                    if self.size >= MAX_PACKET_SIZE {
                        return (Packet::Failed(PacketError::SizeTooBig(self.size as usize)), bytes_read)
                    }
                    println!("Incoming packet size: {} bytes", self.size);
                    (Packet::InProgress(PacketInProgress {
                        received: 0,
                        data: vec![0; self.size as usize],
                    }), bytes_read)
                } else {
                    (Packet::Size(self), bytes_read)
                }
            }
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    (Packet::Size(self), 0)
                } else {
                    println!("{error:#?}");
                    (Packet::Failed(PacketError::StreamError), 0)
                }
            }
        }
    }
}

pub struct PacketInProgress {
    // number of bytes received
    received: usize,
    // actual data
    data: Vec<u8>,
}

impl PacketInProgress {
    fn advance<T>(mut self, stream: &mut T) -> (Packet, usize)
    where
        T: Read,
    {
        if self.received == self.data.len() {
            return (Packet::Received(self.data), 0);
        }

        let slice = unsafe {
            let start = self.data.as_mut_ptr().add(self.received);
            std::slice::from_raw_parts_mut(start, self.data.len() - self.received)
        };

        match stream.read(slice) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    println!("zero bytes received");
                    return (Packet::Failed(PacketError::StreamClosed), 0);
                }

                self.received += bytes_read;

                if self.received < self.data.len() {
                    return (Packet::InProgress(self), bytes_read);
                }

                return (Packet::Received(self.data), bytes_read);
            }
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    return (Packet::InProgress(self), 0);
                }

                println!("{error:#?}");
                (Packet::Failed(PacketError::StreamError), 0)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PacketError {
    StreamError,
    StreamClosed,
    SizeTooBig(usize),
}

impl Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamError => write!(f, "Stream error happened"),
            Self::StreamClosed => write!(f, "Stream closed"),
            Self::SizeTooBig(size) => write!(f, "Packet too big ({size})"),
        }
    }
}

pub enum Packet {
    // In the process of reading size
    Size(PacketReadingSize),

    // Packet is in process of reading
    InProgress(PacketInProgress),

    // Packet was read successfully
    Received(Vec<u8>),

    // Failed to read the packet
    Failed(PacketError),
}

impl Packet {
    pub fn new() -> Packet {
        Packet::Size(PacketReadingSize { size: 0, read: 0 })
    }

    pub fn advance<T>(self, stream: &mut T) -> Packet
    where
        T: Read,
    {
        match self {
            Packet::Size(state) => state.advance(stream).0,
            Packet::InProgress(state) => state.advance(stream).0,
            Packet::Received(_) => self,
            Packet::Failed(_) => self,
        }
    }

    pub fn advance_until_received<T>(self, stream: &mut T) -> Packet
    where
        T: Read,
    {
        let mut packet = self;
        let mut finished = false;
        while !finished {
            packet = match packet {
                Packet::Size(state) => state.advance(stream).0,
                Packet::InProgress(state) => state.advance(stream).0,
                Packet::Received(data) => {
                    finished = true;
                    Packet::Received(data)
                }
                Packet::Failed(err) => {
                    finished = true;
                    println!("{err:#?}");
                    Packet::Failed(err)
                }
            }
        }

        packet
    }

    pub fn advance_until_would_block<T>(self, stream: &mut T) -> Packet
    where
        T: Read,
    {
        let mut packet = self;
        let mut finished = false;
        while !finished {
            packet = match packet {
                Packet::Size(state) => {
                    let (new_state, read_count) = state.advance(stream);
                    if read_count == 0 {
                        finished = true;
                    }
                    new_state
                },
                Packet::InProgress(state) => {
                    let (new_state, read_count) = state.advance(stream);
                    if read_count == 0 {
                        finished = true;
                    }
                    new_state
                }
                Packet::Received(data) => {
                    finished = true;
                    Packet::Received(data)
                }
                Packet::Failed(err) => {
                    finished = true;
                    println!("{err:#?}");
                    Packet::Failed(err)
                }
            }
        }

        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        generate_random_string, make_buffer_for_packet, next_localhost_address,
    };
    use rand::{self, Rng, SeedableRng};
    use std::io::{BufReader, Write};

    #[test]
    fn advance_detailed() {
        let payload = "Hello, world!";
        let buffer = make_buffer_for_packet(payload);
        let mut reader = BufReader::new(&buffer[..]);

        let packet = Packet::new();
        match &packet {
            Packet::Size(state) => {
                assert_eq!(state.read, 0);
                assert_eq!(state.size, 0);
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.advance(&mut reader);
        match &packet {
            Packet::Size(state) => {
                assert_eq!(state.read, 4);
                assert_eq!(state.size, payload.len() as u32);
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.advance(&mut reader);
        match &packet {
            Packet::InProgress(state) => {
                assert_eq!(state.data.len(), payload.len());
                assert_eq!(state.received, 0);
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.advance(&mut reader);
        match packet {
            Packet::Received(data) => {
                assert_eq!(data.len(), payload.len());
                assert_eq!(
                    String::from_utf8(data).expect("Failed to make a string from buffer"),
                    payload
                );
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }
    }

    #[test]
    fn advance_until_received() {
        let payload = "Example string";
        let buffer = make_buffer_for_packet(&payload);
        let mut reader = BufReader::new(&buffer[..]);
        let packet = Packet::new().advance_until_received(&mut reader);

        if let Packet::Received(data) = packet {
            assert_eq!(data.len(), payload.len());
            assert_eq!(
                String::from_utf8(data).expect("Failed to make a string from buffer"),
                payload
            );
        } else {
            panic!("Unexpected state of packet");
        }
    }

    #[test]
    fn read_two_packets_from_same_buffer() {
        let payload_a = "Example string 1";
        let payload_b = "Example string 2";
        let buffer = {
            let mut temp = make_buffer_for_packet(&payload_a);
            temp.extend(make_buffer_for_packet(&payload_b));
            temp
        };
        let mut reader = BufReader::new(&buffer[..]);

        if let Packet::Received(data) = Packet::new().advance_until_received(&mut reader) {
            assert_eq!(data.len(), payload_a.len());
            assert_eq!(
                String::from_utf8(data).expect("Failed to make a string from buffer"),
                payload_a
            );
        } else {
            panic!("Unexpected state of packet");
        }

        if let Packet::Received(data) = Packet::new().advance_until_received(&mut reader) {
            assert_eq!(data.len(), payload_b.len());
            assert_eq!(
                String::from_utf8(data).expect("Failed to make a string from buffer"),
                payload_b
            );
        } else {
            panic!("Unexpected state of packet");
        }
    }

    #[test]
    fn read_packet_with_invalid_size() {
        let payload = "Example string";
        let buffer = make_buffer_for_packet(&payload);
        let mut reader = BufReader::new(&buffer[0..5]);

        if let Packet::Failed(error) = Packet::new().advance_until_received(&mut reader) {
            assert_eq!(error, PacketError::StreamClosed {});
        } else {
            panic!("Unexpected state of packet");
        }
    }

    #[test]
    fn advance_until_received_tcp() {
        let payload = generate_random_string(1234, 2000, 10000);
        let address = next_localhost_address();

        let join_handle = {
            // clone values before they move to spawned thread
            let payload = payload.clone();
            let address = address.clone();
            std::thread::spawn(move || {
                let listener = std::net::TcpListener::bind(address).unwrap();
                match listener.incoming().next().expect("") {
                    Ok(mut stream) => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        let buffer = make_buffer_for_packet(&payload);
                        let mut sent_bytes = 0;
                        let mut part_size_generator = rand::rngs::StdRng::seed_from_u64(1234);
                        while sent_bytes < buffer.len() {
                            let part_size = part_size_generator.gen_range(1..200);
                            stream
                                .write_all(
                                    &buffer[sent_bytes
                                        ..std::cmp::min(buffer.len(), sent_bytes + part_size)],
                                )
                                .unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            sent_bytes += part_size;
                        }

                        // read something so that socket is not closed too early
                        let mut read_data = String::new();
                        stream.read_to_string(&mut read_data).unwrap();
                    }
                    Err(e) => {
                        eprintln!("failed to accept client connection: {}", e);
                    }
                }
            })
        };

        {
            let stream = std::net::TcpStream::connect(address).unwrap();
            stream
                .set_nonblocking(true)
                .expect("Can't make stream nonblocking");
            let mut reader = BufReader::new(stream);
            let packet = Packet::new().advance_until_received(&mut reader);

            if let Packet::Received(data) = packet {
                assert_eq!(data.len(), payload.len());
                assert_eq!(
                    String::from_utf8(data).expect("Failed to make a string from buffer"),
                    payload
                );
            } else {
                panic!("Unexpected state of packet");
            }
        }

        join_handle.join().unwrap()
    }
}
