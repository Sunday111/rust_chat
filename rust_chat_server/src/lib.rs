use rust_chat;
use rust_chat::ChatResult;
use rust_chat::Connection;
use rust_chat::ConvertibleToChatResult;
use rust_chat::MesasgeFromUser;
use rust_chat::CommandType;
use std::net::TcpListener;

struct ChatServer {
    connection_listener: TcpListener,
    connections: Vec<Option<Connection>>,
    messages_from_user: Vec<MesasgeFromUser>,
}

impl ChatServer {
    pub fn new<'a>(address: &str) -> ChatResult<ChatServer> {
        let tcp_listener = TcpListener::bind(address).to_chat_result()?;
        tcp_listener.set_nonblocking(true).to_chat_result()?;
        Ok(ChatServer {
            connection_listener: tcp_listener,
            connections: Vec::new(),
            messages_from_user: Vec::new(),
        })
    }

    pub fn tick(&mut self) {
        self.accept_connections();
        self.receive_data();
        let commands = self.gather_commands();
        self.handle_commands(commands);
        self.send_messages();
        self.send_data();
        self.remove_closed_connections();
    }

    pub fn accept_connections(&mut self) {
        loop {
            match self.connection_listener.accept() {
                Ok((stream, _)) => {
                    stream
                        .set_nonblocking(true)
                        .expect("Failed to make tcp stream non-blocking");
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

    pub fn handle_commands(&mut self, commands: Vec<Vec<u8>>) {
        for command in commands {
            self.handle_command(command);
        }
    }

    pub fn send_messages(&mut self) {
        for message_from_user in &self.messages_from_user {
            let message_str = {
                let mut object = serde_json::value::Map::new();
                object.insert(
                    "Type".to_string(),
                    serde_json::to_value(CommandType::MessageFromUser).unwrap(),
                );
                object.insert(
                    "Data".to_string(),
                    serde_json::to_value(message_from_user)
                    .expect("fail"),
                );

                serde_json::to_string(&object).unwrap()
            };

            for opt_connection in &mut self.connections {
                let mut connection = opt_connection.take().unwrap();
                if let Connection::Established(state) = &mut connection {
                    state.enqueue_message(message_str.clone());
                }

                *opt_connection = Some(connection);
            }
        }

        self.messages_from_user.clear();
    }

    pub fn handle_command(&mut self, command: Vec<u8>) {
        type JsonValue = serde_json::Value;
        let mut cmd_json = match serde_json::from_slice::<JsonValue>(&command) {
            Ok(cmd_json) => cmd_json,
            Err(parse_err) => {
                println!(
                    "Failed to parse command json: {}. Error: {}.",
                    String::from_utf8_lossy(&command),
                    parse_err
                );
                return;
            }
        };

        let cmd_json = match cmd_json.as_object_mut() {
            Some(map) => map,
            None => {
                println!("Command json expected to be object");
                return;
            }
        };

        let key_command_type = "Type";
        let command_type = match cmd_json.remove(key_command_type) {
            Some(cmd_type_str) => match serde_json::from_value::<CommandType>(cmd_type_str) {
                Ok(command_type) => command_type,
                Err(err) => {
                    println!("Failed to parse command type: {}", err);
                    return;
                }
            },
            None => {
                println!("Invalid json - {key_command_type} not found.");
                return;
            }
        };

        let key_commad_data = "Data";
        let cmd_data_value = match cmd_json.remove(key_commad_data) {
            Some(cmd_data_value) => cmd_data_value,
            None => {
                println!("Invalid json - {key_commad_data} not found.");
                return;
            }
        };

        match command_type {
            CommandType::MessageFromUser => {
                match serde_json::from_value::<MesasgeFromUser>(cmd_data_value) {
                    Ok(message_from_user) => self.messages_from_user.push(message_from_user),
                    Err(err) => {
                        println!("Failed to parse message from user. {}", err);
                        return;
                    }
                }
            }
        }
    }

    pub fn send_data(&mut self) {
        for opt_connection in &mut self.connections {
            let connection = opt_connection.take().unwrap();
            *opt_connection = Some(connection.send());
        }
    }

    pub fn gather_commands(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();

        for connection in &mut self.connections {
            if let Connection::Established(state) = connection.as_mut().unwrap() {
                if let Some(message) = state.take_message() {
                    messages.push(message);
                }
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
