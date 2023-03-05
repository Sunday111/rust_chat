use std::str::FromStr;

use crate::client::Client;
use crate::client::WaitingForConnectionInfoState;

pub struct Application {
    client: Option<Client>,
}

impl Default for Application {
    fn default() -> Self {
        Application {
            client: Some(Client::WaitingForConnectionInfo(
                WaitingForConnectionInfoState::new(),
            )),
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.client = Some(match self.client.take().unwrap() {
                Client::WaitingForConnectionInfo(mut state) => {
                    let can_parse_address = std::net::SocketAddr::from_str(&state.address).is_ok();
                    ui.heading("Connection info");
                    ui.horizontal(|ui| {
                        let name_label = ui.label("Server address: ");
                        ui.text_edit_singleline(&mut state.address)
                            .labelled_by(name_label.id);

                        if !can_parse_address {
                            let error_text =
                                egui::RichText::new("Inavlid address").color(egui::Color32::RED);
                            ui.label(error_text);
                        }
                    });

                    let connect_button =
                        ui.add_enabled(can_parse_address, egui::Button::new("Connect"));
                    if connect_button.clicked() {
                        state.connect()
                    } else {
                        Client::WaitingForConnectionInfo(state)
                    }
                }
                Client::Connected(state) => {
                    ui.heading("Connected");
                    state.begin_login()
                }
                Client::WaitingForLoginInfo(mut state) => {
                    ui.heading("Login info");
                    ui.horizontal(|ui| {
                        let name_label = ui.label("Username: ");
                        ui.text_edit_singleline(&mut state.login_info.user)
                            .labelled_by(name_label.id);
                    });
                    if ui.button("Login").clicked() {
                        state.login()
                    } else {
                        Client::WaitingForLoginInfo(state)
                    }
                }
                Client::ConnectionFailed(state) => {
                    ui.heading("Login failed");
                    ui.colored_label(egui::Color32::RED, state.reason.to_string());
                    if ui.button("To connection page").clicked() {
                        Client::WaitingForConnectionInfo(WaitingForConnectionInfoState {
                            address: state.connection_info.address.to_string(),
                        })
                    } else {
                        Client::ConnectionFailed(state)
                    }
                }
                Client::LoggedIn(mut state) => {
                    ui.heading("Welcome");
                    let scroll_area = egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .auto_shrink([false; 2]);
                    scroll_area
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for message in &state.received_messages {
                                    ui.colored_label(egui::Color32::YELLOW, message);
                                }
                            });
                        })
                        .inner;

                    ui.horizontal(|ui| {
                        let message_label = ui.label("Message: ");
                        ui.text_edit_singleline(&mut state.current_input)
                            .labelled_by(message_label.id);
                        if ui.button("Send").clicked() {
                            state.send_message();
                            state.current_input.clear();
                        }
                    });

                    state.tick()
                }
                Client::LoginFailed(state) => {
                    ui.heading("Login failed");
                    ui.colored_label(egui::Color32::RED, state.reason.to_string());
                    if ui.button("To connection page").clicked() {
                        Client::WaitingForConnectionInfo(WaitingForConnectionInfoState {
                            address: state.connection_info.address.to_string(),
                        })
                    } else {
                        Client::LoginFailed(state)
                    }
                }
                Client::Disconnected(state) => {
                    ui.heading("Login failed");
                    ui.colored_label(egui::Color32::RED, state.reason.to_string());
                    if ui.button("To connection page").clicked() {
                        Client::WaitingForConnectionInfo(WaitingForConnectionInfoState {
                            address: state.connection_info.address.to_string(),
                        })
                    } else {
                        Client::Disconnected(state)
                    }
                }
            });
        });
    }
}
