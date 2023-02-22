use std::io::Read;

const MAX_PACKET_SIZE:u32 = 65536;

pub struct PacketReadingSize {
    size: u32,
    read: usize,
}

impl PacketReadingSize {
    fn progress<T>(mut self, stream: &mut T) -> Packet
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
    fn progress<T>(mut self, stream: &mut T) -> Packet
    where
        T: Read,
    {
        if self.received == self.data.len() {
            return Packet::Complete(self.data)
        }

        let slice = unsafe {
            let start = self.data.as_mut_ptr().add(self.received);
            std::slice::from_raw_parts_mut(start, self.data.len() - self.received)
        };

        match stream.read(slice) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    return Packet::Failed(PacketError::StreamClosed)
                }

                self.received += bytes_read;
                return Packet::InProgress(self)
            },
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    return Packet::InProgress(self)
                }

                Packet::Failed(PacketError::StreamError)
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

    pub fn progress<T>(self, stream: &mut T) -> Packet
    where
        T: Read,
    {
        match self {
            Packet::Size(state) => state.progress(stream),
            Packet::InProgress(state) => state.progress(stream),
            Packet::Complete(_) => self,
            Packet::Failed(_) => self,
        }
    }

    pub fn progress_loop<T>(self, stream: &mut T) -> Packet
        where
            T: Read,
    {
        let mut packet = self;
        let mut finished = false;
        while !finished {
            packet = match packet {
                Packet::Size(state) => state.progress(stream),
                Packet::InProgress(state) => state.progress(stream),
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
    use std::{io::{BufReader, Write}, os::windows::thread};

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

        let packet = packet.progress(&mut reader);
        match &packet {
            Packet::Size(state) => {
                assert_eq!(state.read, 4);
                assert_eq!(state.size, payload.len() as u32);
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.progress(&mut reader);
        match &packet {
            Packet::InProgress(state) => {
                assert_eq!(state.data.len(), payload.len());
                assert_eq!(state.received, 0);
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.progress(&mut reader);
        match &packet {
            Packet::InProgress(state) => {
                assert_eq!(state.data.len(), payload.len());
                assert_eq!(state.received, payload.len());
            }
            _ => {
                panic!("Unexpected state of packet");
            }
        }

        let packet = packet.progress(&mut reader);
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
    fn progress_loop() {
        let payload = "Example string";
        let buffer = make_buffer_for_packet(&payload);
        let mut reader = BufReader::new(&buffer[..]);
        let packet = Packet::new().progress_loop(&mut reader);

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

        if let Packet::Complete(data) = Packet::new().progress_loop(&mut reader) {
            assert_eq!(data.len(), payload_a.len());
            assert_eq!(
                String::from_utf8(data).expect("Failed to make a string from buffer"),
                payload_a
            );
        } else {
            panic!("Unexpected state of packet");
        }

        if let Packet::Complete(data) = Packet::new().progress_loop(&mut reader) {
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

        if let Packet::Failed(error) = Packet::new().progress_loop(&mut reader) {
            assert_eq!(error, PacketError::StreamClosed{});
        } else {
            panic!("Unexpected state of packet");
        }
    }

    #[test]
    fn progress_loop_from_tcp() {
        let payload = "Hello, tcp stream!";

        let join_handle = {
            let payload = payload.clone();
            std::thread::spawn(move || {
                let listener = std::net::TcpListener::bind("127.0.0.1:5432").unwrap();
                match listener.incoming().next().expect("") {
                    Ok(mut stream) => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        let buffer = make_buffer_for_packet(&payload);

                        let mut k = 0;
                        while k < buffer.len() {
                            stream.write_all(&buffer[k..k+2]).unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            k += 2;
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
            let stream = std::net::TcpStream::connect("127.0.0.1:5432").unwrap();
            stream.set_nonblocking(true).expect("Can't make stream nonblocking");
            let mut reader = BufReader::new(stream);
            let packet = Packet::new().progress_loop(&mut reader);
    
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
