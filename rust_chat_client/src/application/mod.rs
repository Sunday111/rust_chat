use std::str::FromStr;

use crate::client::Client;
use crate::client::LoggedInState;
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

impl Application {
    fn gather_connection_info_page(
        &mut self,
        ctx: &egui::Context,
        mut state: WaitingForConnectionInfoState,
    ) -> Client {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let can_parse_address = std::net::SocketAddr::from_str(&state.address).is_ok();
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
            })
            .inner
    }

    fn chat_page(&mut self, ctx: &egui::Context, mut state: LoggedInState) -> Client {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let message_label = ui.label("Message: ");
                    ui.text_edit_singleline(&mut state.current_input)
                        .labelled_by(message_label.id);
                    if ui.button("Send").clicked() {
                        state.send_message();
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for message in &state.received_messages {
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::GREEN, &message.username);
                                ui.separator();
                                ui.colored_label(egui::Color32::WHITE, &message.text);
                            });
                        }
                    });
                });
        });

        state.tick()
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.client = Some(match self.client.take().unwrap() {
            Client::WaitingForConnectionInfo(state) => self.gather_connection_info_page(ctx, state),
            Client::Connected(state) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        ui.heading("Connected");
                        state.begin_login()
                    })
                    .inner
            }
            Client::WaitingForLoginInfo(mut state) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
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
                    })
                    .inner
            }
            Client::ConnectionFailed(state) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        ui.heading("Login failed");
                        ui.colored_label(egui::Color32::RED, state.reason.to_string());
                        if ui.button("To connection page").clicked() {
                            Client::WaitingForConnectionInfo(WaitingForConnectionInfoState {
                                address: state.connection_info.address.to_string(),
                            })
                        } else {
                            Client::ConnectionFailed(state)
                        }
                    })
                    .inner
            }
            Client::LoggedIn(state) => self.chat_page(ctx, state),
            Client::LoginFailed(state) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        ui.heading("Login failed");
                        ui.colored_label(egui::Color32::RED, state.reason.to_string());
                        if ui.button("To connection page").clicked() {
                            Client::WaitingForConnectionInfo(WaitingForConnectionInfoState {
                                address: state.connection_info.address.to_string(),
                            })
                        } else {
                            Client::LoginFailed(state)
                        }
                    })
                    .inner
            }
            Client::Disconnected(state) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        ui.heading("Login failed");
                        ui.colored_label(egui::Color32::RED, state.reason.to_string());
                        if ui.button("To connection page").clicked() {
                            Client::WaitingForConnectionInfo(WaitingForConnectionInfoState {
                                address: state.connection_info.address.to_string(),
                            })
                        } else {
                            Client::Disconnected(state)
                        }
                    })
                    .inner
            }
        });
    }
}
