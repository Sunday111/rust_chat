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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                },
                Client::Connected(mut state) => {
                    ui.heading("Connection info");
                    ui.horizontal(|ui| {
                        let address_label = ui.label("Server address: ");
                        ui.text_edit_singleline(&mut state.connection_info.address)
                            .labelled_by(address_label.id);
                    });
                    state.begin_login()
                },
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
                },
                Client::ConnectionFailed(state) => {
                    panic!("Connection failed");
                    Client::ConnectionFailed(state)
                },
                Client::LoggedIn(mut state) => {
                    ui.heading("Connection info");
                    ui.horizontal(|ui| {
                        let address_label = ui.label("Server address: ");
                        ui.text_edit_singleline(&mut state.current_input)
                            .labelled_by(address_label.id);
                    });
                    if ui.button("Send").clicked() {
                        state.current_input.clear();
                    }
                    Client::LoggedIn(state)
                },
                Client::LoginFailed(state) => {
                    panic!("Connection failed");
                    Client::LoginFailed(state)
                },
                Client::Disconnected(state) => {
                    panic!("Connection failed");
                    Client::Disconnected(state)
                },
            });
        });
    }
}
