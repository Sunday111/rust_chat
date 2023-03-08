use std::{net::TcpStream, str::FromStr, mem::swap};

use rust_chat::{
    CommandType, ConnectionInfo, LoginInfo, MesasgeFromUser, PacketReceiver, PacketSender,
};

pub fn try_connect(connection_info: ConnectionInfo) -> Client {
    match TcpStream::connect(&connection_info.address) {
        Ok(stream) => {
            stream
                .set_nonblocking(true)
                .expect("Failed to make stream non-blocking");
            Client::Connected(ConnectedState {
                connection_info: connection_info,
                stream: stream,
            })
        }
        Err(err) => Client::ConnectionFailed(ConnectionFailedState {
            connection_info: connection_info,
            reason: err.to_string(),
        }),
    }
}

//---------------------------------------------------------------------------------------------------

pub struct WaitingForConnectionInfoState {
    pub address: String,
}

impl WaitingForConnectionInfoState {
    pub fn connect(self) -> Client {
        if let Ok(address) = std::net::SocketAddr::from_str(&self.address) {
            let connection_info = ConnectionInfo { address: address };
            try_connect(connection_info)
        } else {
            Client::WaitingForConnectionInfo(self)
        }
    }

    pub fn new() -> WaitingForConnectionInfoState {
        WaitingForConnectionInfoState {
            address: "127.0.0.1:8787".to_string(),
        }
    }
}

//---------------------------------------------------------------------------------------------------

pub struct ConnectionFailedState {
    pub connection_info: ConnectionInfo,
    pub reason: String,
}

impl ConnectionFailedState {
    pub fn retry(self) -> Client {
        try_connect(self.connection_info)
    }
}

//---------------------------------------------------------------------------------------------------

pub struct ConnectedState {
    pub connection_info: ConnectionInfo,
    pub stream: TcpStream,
}

impl ConnectedState {
    pub fn begin_login(self) -> Client {
        Client::WaitingForLoginInfo(WaitingForLoginInfoState {
            connection_info: self.connection_info,
            login_info: LoginInfo {
                user: "".to_string(),
            },
            stream: self.stream,
            sender: PacketSender::new(),
        })
    }
}

//---------------------------------------------------------------------------------------------------

pub struct WaitingForLoginInfoState {
    pub connection_info: ConnectionInfo,
    pub login_info: LoginInfo,
    pub stream: TcpStream,
    pub sender: PacketSender,
}

impl WaitingForLoginInfoState {
    pub fn login(mut self) -> Client {
        let login_message = &format!("{{ \"username\": \"{}\" }}", self.login_info.user);
        let mut data = Vec::new();
        data.extend_from_slice(&login_message.as_bytes());
        self.sender.add_to_send_queue(data);

        while !self.sender.empty() {
            if let Err(err) = self.sender.advance(&mut self.stream) {
                return Client::LoginFailed(LoginFailedState {
                    connection_info: self.connection_info,
                    login_info: self.login_info,
                    reason: err.to_string(),
                });
            }
        }

        Client::LoggedIn(LoggedInState {
            connection_info: self.connection_info,
            login_info: self.login_info,
            stream: self.stream,
            sender: PacketSender::new(),
            receiver: PacketReceiver::new(),
            current_input: String::new(),
            received_messages: Vec::new(),
        })
    }
}

//---------------------------------------------------------------------------------------------------

pub struct LoggedInState {
    connection_info: ConnectionInfo,
    login_info: LoginInfo,
    stream: TcpStream,
    sender: PacketSender,
    receiver: PacketReceiver,
    pub current_input: String,
    pub received_messages: Vec<MesasgeFromUser>,
}

impl LoggedInState {
    pub fn send_message(&mut self) {
        if self.current_input.is_empty() {
            return;
        }

        let mut object = serde_json::value::Map::new();
        object.insert(
            "Type".to_string(),
            serde_json::to_value(CommandType::MessageFromUser).unwrap(),
        );

        let mut current_message = String::new();
        swap(&mut current_message, &mut self.current_input);
        object.insert(
            "Data".to_string(),
            serde_json::to_value(MesasgeFromUser {
                username: self.login_info.user.clone(),
                text: current_message,
            })
            .expect("fail"),
        );

        let buf = serde_json::to_string(&object).unwrap();

        self.sender.add_to_send_queue(Vec::from(buf.as_bytes()));
        self.current_input.clear();
    }

    fn take_message(&mut self) -> Option<String> {
        if let Some(packet) = self.receiver.pop_packet() {
            Some(String::from_utf8(packet).expect(""))
        } else {
            None
        }
    }

    pub fn tick(mut self) -> Client {
        if let Err(err) = self.sender.advance(&mut self.stream) {
            return Client::Disconnected(DisconnectedState {
                connection_info: self.connection_info,
                login_info: self.login_info,
                reason: err.to_string(),
            });
        }

        if let Err(err) = self.receiver.advance(&mut self.stream) {
            return Client::Disconnected(DisconnectedState {
                connection_info: self.connection_info,
                login_info: self.login_info,
                reason: err.to_string(),
            });
        }

        while let Some(data) = self.take_message() {
            let mut cmd_json = match serde_json::from_slice::<serde_json::Value>(data.as_bytes()) {
                Ok(cmd_json) => cmd_json,
                Err(parse_err) => {
                    println!(
                        "Failed to parse command json: {}. Error: {}.",
                        data, parse_err
                    );
                    continue;
                }
            };

            let cmd_json = match cmd_json.as_object_mut() {
                Some(map) => map,
                None => {
                    println!("Command json expected to be object");
                    continue;
                }
            };

            let key_command_type = "Type";
            let command_type = match cmd_json.remove(key_command_type) {
                Some(cmd_type_str) => match serde_json::from_value::<CommandType>(cmd_type_str) {
                    Ok(command_type) => command_type,
                    Err(err) => {
                        println!("Failed to parse command type: {}", err);
                        continue;
                    }
                },
                None => {
                    println!("Invalid json - {key_command_type} not found.");
                    continue;
                }
            };

            let key_commad_data = "Data";
            let cmd_data_value = match cmd_json.remove(key_commad_data) {
                Some(cmd_data_value) => cmd_data_value,
                None => {
                    println!("Invalid json - {key_commad_data} not found.");
                    continue;
                }
            };

            match command_type {
                CommandType::MessageFromUser => {
                    match serde_json::from_value::<MesasgeFromUser>(cmd_data_value) {
                        Ok(message_from_user) => self.received_messages.push(message_from_user),
                        Err(err) => {
                            println!("Failed to parse message from user. {}", err);
                            continue;
                        }
                    }
                }
            }
        }

        Client::LoggedIn(self)
    }
}

//---------------------------------------------------------------------------------------------------

pub struct LoginFailedState {
    pub connection_info: ConnectionInfo,
    pub login_info: LoginInfo,
    pub reason: String,
}

//---------------------------------------------------------------------------------------------------

pub struct DisconnectedState {
    pub connection_info: ConnectionInfo,
    pub login_info: LoginInfo,
    pub reason: String,
}

//---------------------------------------------------------------------------------------------------

pub enum Client {
    WaitingForConnectionInfo(WaitingForConnectionInfoState),
    Connected(ConnectedState),
    WaitingForLoginInfo(WaitingForLoginInfoState),
    ConnectionFailed(ConnectionFailedState),
    LoggedIn(LoggedInState),
    LoginFailed(LoginFailedState),
    Disconnected(DisconnectedState),
}
