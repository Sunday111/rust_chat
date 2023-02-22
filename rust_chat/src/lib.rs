mod chat_result;
mod connection;
mod packet;

use std::net::SocketAddr;
use std::net::TcpStream;

pub use chat_result::ChatError;
pub use chat_result::ChatResult;
pub use chat_result::ConvertibleToChatResult;
pub use connection::Connection;

#[derive(Debug)]
pub enum Answer {
    Yes,
    No,
}
