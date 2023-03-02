use rust_chat;
use rust_chat::ChatResult;
use rust_chat::Connection;
use rust_chat::ConvertibleToChatResult;
use std::net::TcpListener;

struct ChatServer {
    connection_listener: TcpListener,
    connections: Vec<Option<Connection>>,
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
        self.receive_data();
        self.remove_closed_connections();
        let messages = self.gather_messages();
        self.send_data(messages);
        self.remove_closed_connections();
    }

    pub fn accept_connections(&mut self) {
        loop {
            match self.connection_listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(true).expect("Failed to make tcp stream non-blocking");
                    let new_connection = Connection::new(stream);
                    self.connections.push(Some(new_connection));
                }
                Err(error) => {
                    if error.kind() == std::io::ErrorKind::WouldBlock {
                        break;
                    }
                }
            }
        }
    }

    pub fn receive_data(&mut self) {
        for opt_connection in &mut self.connections {
            let connection = opt_connection.take().unwrap();
            *opt_connection = Some(connection.receive());
        }
    }

    pub fn send_data(&mut self, messages: Vec<String>) {
        for opt_connection in &mut self.connections {
            let mut connection = opt_connection.take().unwrap();
            for message in &messages {
                connection.enqueue_message(message.clone());
            }
            
            *opt_connection = Some(connection.send());
        }
    }

    pub fn gather_messages(&mut self) -> Vec<String> {
        let mut messages = Vec::new();

        for connection in &mut self.connections {
            if let Some(message) = connection.as_mut().unwrap().take_message() {
                messages.push(message);
            }
        }

        messages
    }

    fn remove_closed_connections(&mut self) {
        // remove closed connections
        self.connections.retain(|opt_connection| {
            if let Connection::Closed(_) = opt_connection.as_ref().unwrap() {
                println!("Connection closed");
                return false;
            }

            return true;
        })
    }
}

pub fn run_app() -> ChatResult<()> {
    let mut server = ChatServer::new("127.0.0.1:8787")?;
    loop {
        server.tick();
    }
}
