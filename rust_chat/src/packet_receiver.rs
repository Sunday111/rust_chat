use std::io::Read;
use std::collections::VecDeque;

use crate::incoming_packet::Packet;
use crate::incoming_packet::PacketError;

pub struct PacketReceiver {
    received: VecDeque<Vec<u8>>,
    current: Option<Packet>,
}

impl PacketReceiver {
    pub fn new() -> PacketReceiver {
        PacketReceiver {
            received: VecDeque::new(),
            current: Some(Packet::new()),
        }
    }

    pub fn advance<Stream>(&mut self, stream: &mut Stream) -> std::result::Result<(), PacketError>
    where
        Stream: Read,
    {
        let mut packet = self.current.take().unwrap().advance_until_would_block(stream);
        match packet {
            Packet::Size(state) => {
                packet = Packet::Size(state)
            }
            Packet::InProgress(state) => {
                packet = Packet::InProgress(state)
            },
            Packet::Received(data) => {
                self.received.push_back(data);
                packet = Packet::new();
            },
            Packet::Failed(err) => {
                return Err(err);
            },
        }

        self.current = Some(packet);
        Ok(())
    }

    pub fn pop_packet(&mut self) -> Option<Vec<u8>> {
        self.received.pop_back()
    }
}
