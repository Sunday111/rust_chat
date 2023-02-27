use rust_chat;
use rust_chat::ChatResult;
use rust_chat::Connection;
use rust_chat::ConvertibleToChatResult;
use std::net::TcpListener;

struct ChatServer {
    connection_listener: TcpListener,
    connections: Vec<Connection>,
}

impl ChatServer {
    pub fn new<'a>(address: &str) -> ChatResult<ChatServer> {
        let tcp_listener = TcpListener::bind(address).to_chat_result()?;
        tcp_listener.set_nonblocking(true).to_chat_result()?;
        Ok(ChatServer {
            connection_listener: tcp_listener,
            connections: Vec::new(),
        })
    }

    pub fn tick(&mut self) {
        self.accept_connections();
    }

    pub fn accept_connections(&mut self) {
        loop {
            match self.connection_listener.accept() {
                Ok((stream, _)) => {
                    let new_connection = Connection::new(stream);
                    self.connections.push(new_connection);
                }
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::WouldBlock {
                        break;
                    }
                }
            }
        }
    }
}

pub fn run_app() -> ChatResult<()> {
    let mut server = ChatServer::new("127.0.0.1:8787")?;
    loop {
        server.tick();
    }
}
