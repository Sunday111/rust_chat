mod chat_result;
mod connection;
mod incoming_packet;
mod outgoing_packet;
mod test_utils;

pub use chat_result::ChatError;
pub use chat_result::ChatResult;
pub use chat_result::ConvertibleToChatResult;
pub use connection::Connection;

#[derive(Debug)]
pub enum Answer {
    Yes,
    No,
}
