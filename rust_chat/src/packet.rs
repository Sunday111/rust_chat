use std::io::Read;

const MAX_PACKET_SIZE:u32 = 65536;

pub struct PacketReadingSize {
    size: u32,
    read: usize,
}

impl PacketReadingSize {
    fn advance<T>(mut self, stream: &mut T) -> Packet
    where
        T: Read,
    {
        if self.read < 4 {
            let remaining_buf = unsafe {
                let pointer = (&mut self.size as *mut _ as *mut u8).add(self.read);
                std::slice::from_raw_parts_mut(pointer, 4 - self.read)
            };
            match stream.read(remaining_buf) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        return Packet::Failed(PacketError::StreamClosed);
                    }

                    assert!(self.read + bytes_read <= 4);
                    self.read += bytes_read;
                    Packet::Size(self)
                }
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::WouldBlock {
                        Packet::Size(self)
                    } else {
                        Packet::Failed(PacketError::StreamError)
                    }
                }
            }
        } else {
            if self.size > MAX_PACKET_SIZE {
                return Packet::Failed(PacketError::SizeTooBig(self.size as usize))
            }

            Packet::InProgress(PacketInProgress {
                received: 0,
                data: vec![0; self.size as usize],
            })
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
            return (Packet::Complete(self.data), 0)
        }

        let slice = unsafe {
            let start = self.data.as_mut_ptr().add(self.received);
            std::slice::from_raw_parts_mut(start, self.data.len() - self.received)
        };

        match stream.read(slice) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    return (Packet::Failed(PacketError::StreamClosed), 0)
                }

                self.received += bytes_read;

                if self.received < self.data.len() {
                    return (Packet::InProgress(self), bytes_read)
                }

                return (Packet::Complete(self.data), bytes_read)
            },
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    return (Packet::InProgress(self), 0)
                }

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

pub enum Packet {
    // In the process of reading size
    Size(PacketReadingSize),

    // Packet is in process of reading
    InProgress(PacketInProgress),

    // Packet was read successfully
    Complete(Vec<u8>),

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
            Packet::Size(state) => state.advance(stream),
            Packet::InProgress(state) => state.advance(stream).0,
            Packet::Complete(_) => self,
            Packet::Failed(_) => self,
        }
    }

    pub fn advance_to_complete<T>(self, stream: &mut T) -> Packet
        where
            T: Read,
    {
        let mut packet = self;
        let mut finished = false;
        while !finished {
            packet = match packet {
                Packet::Size(state) => state.advance(stream),
                Packet::InProgress(state) => state.advance(stream).0,
                Packet::Complete(data) => {
                    finished = true;
                    Packet::Complete(data)
                },
                Packet::Failed(err) => {
                    finished = true;
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
                Packet::Size(state) => state.advance(stream),
                Packet::InProgress(state) => {
                    let (new_state, read_count) = state.advance(stream);
                    if read_count == 0 {
                        finished = true;
                    }
                    new_state
                } ,
                Packet::Complete(data) => {
                    finished = true;
                    Packet::Complete(data)
                },
                Packet::Failed(err) => {
                    finished = true;
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
    use std::{io::{BufReader, Write}};
    use rand::{Rng, SeedableRng};
    use rand;

    static PORT_COUNTER: std::sync::Mutex<std::cell::RefCell<i32>> = std::sync::Mutex::new(std::cell::RefCell::new(5432));

    fn get_next_port() -> i32 {
        let guard = PORT_COUNTER.lock().expect("");
        let mut value_ref = guard.borrow_mut();
        let previous_value = *value_ref;
        *value_ref += 1;
        previous_value
    }

    fn next_localhost_address() -> String {
        format!("127.0.0.1:{}", get_next_port())
    }

    fn make_buffer_for_packet(payload:&str) -> Vec<u8> {
        assert_ne!(payload.len(), 0);
        let len: u32 = payload.len().try_into().expect("");
        let slice = unsafe {
            let pointer = &len as *const _ as *const u8;
            std::slice::from_raw_parts(pointer, 4)
        };
        let mut buffer: Vec<u8> = Vec::new();
        buffer
            .write(&slice)
            .expect("Failed to write size to buffer");
        buffer
            .write(payload.as_bytes())
            .expect("Failed to write payload to buffer");
        buffer
    }

    #[test]
    fn detailed_packet_progress() {
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
        match &packet {
            Packet::InProgress(state) => {
                assert_eq!(state.data.len(), payload.len());
                assert_eq!(state.received, payload.len());
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.advance(&mut reader);
        match packet {
            Packet::Complete(data) => {
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
    fn advance_blocking() {
        let payload = "Example string";
        let buffer = make_buffer_for_packet(&payload);
        let mut reader = BufReader::new(&buffer[..]);
        let packet = Packet::new().advance_to_complete(&mut reader);

        if let Packet::Complete(data) = packet {
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

        if let Packet::Complete(data) = Packet::new().advance_to_complete(&mut reader) {
            assert_eq!(data.len(), payload_a.len());
            assert_eq!(
                String::from_utf8(data).expect("Failed to make a string from buffer"),
                payload_a
            );
        } else {
            panic!("Unexpected state of packet");
        }

        if let Packet::Complete(data) = Packet::new().advance_to_complete(&mut reader) {
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

        if let Packet::Failed(error) = Packet::new().advance_to_complete(&mut reader) {
            assert_eq!(error, PacketError::StreamClosed{});
        } else {
            panic!("Unexpected state of packet");
        }
    }

    fn generate_random_string(seed:u64, min_length: usize, max_length: usize) -> String {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let length = rng.gen_range(min_length..max_length + 1);
    
        (0..length)
            .map(|_| rng.gen_range(b'a'..b'z' + 1) as char)
            .collect()
    }

    #[test]
    fn progress_loop_from_tcp() {
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

                        println!("First bytes in buffer: {:?}", &buffer[0..4]);

                        let mut k = 0;
                        let mut part_size_generator = rand::rngs::StdRng::seed_from_u64(1234);
                        while k < buffer.len() {
                            let part_size = part_size_generator.gen_range(1..200);
                            stream.write_all(&buffer[k..std::cmp::min(buffer.len(), k + part_size)]).unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            k += part_size;
                        }

                        // read something so that socket is not closed too earlier
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
            stream.set_nonblocking(true).expect("Can't make stream nonblocking");
            let mut reader = BufReader::new(stream);
            let packet = Packet::new().advance_to_complete(&mut reader);
    
            if let Packet::Complete(data) = packet {
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
