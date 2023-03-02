use std::io::Write;
use std::collections::VecDeque;

use crate::outgoing_packet::Packet;
use crate::outgoing_packet::PacketError;

pub struct PacketSender {
    send_queue: VecDeque<Vec<u8>>,
    current: Option<Packet>,
}

impl PacketSender {
    pub fn new() -> PacketSender {
        PacketSender {
            send_queue: VecDeque::new(),
            current: None,
        }
    }

    pub fn advance<Stream>(&mut self, stream: &mut Stream) -> std::result::Result<(), PacketError>
    where
        Stream: Write,
    {
        let packet = match self.current.take() {
            Some(packet) => packet,
            None => {
                match self.send_queue.pop_front() {
                    Some(data) => Packet::new(&data),
                    None => return Ok(())
                }
            },
        };

        let packet = packet.advance_until_would_block(stream);
        match packet {
            Packet::InProgress(in_progress) => {
                self.current = Some(Packet::InProgress(in_progress));
                Ok(())
            }
            Packet::Sent => Ok(()),
            Packet::Failed(err) => Err(err),
        }
    }

    pub fn add_to_send_queue(&mut self, data: Vec<u8>) {
        self.send_queue.push_back(data);
    }
}
