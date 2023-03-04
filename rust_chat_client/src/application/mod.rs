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
                    ui.heading("Connection info");
                    ui.horizontal(|ui| {
                        let name_label = ui.label("Server address: ");
                        ui.text_edit_singleline(&mut state.connection_info.address)
                            .labelled_by(name_label.id);
                    });
                    if ui.button("Connect").clicked() {
                        state.connect()
                    } else {
                        Client::WaitingForConnectionInfo(state)
                    }
                }
                Client::Connected(mut state) => {
                    ui.heading("Connection info");
                    ui.horizontal(|ui| {
                        let address_label = ui.label("Server address: ");
                        ui.text_edit_singleline(&mut state.connection_info.address)
                            .labelled_by(address_label.id);
                    });
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
                            connection_info: state.connection_info,
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
                            connection_info: state.connection_info,
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
                            connection_info: state.connection_info,
                        })
                    } else {
                        Client::Disconnected(state)
                    }
                }
            });
        });
    }
}
