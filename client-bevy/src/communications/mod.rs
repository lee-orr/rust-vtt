use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

#[cfg(feature = "native")]
pub mod server;
pub mod shared;
#[cfg(feature = "native")]
use server::*;

pub mod client;

use client::*;
use shared::*;

pub struct CommunicationsPlugin;

const DEFAULT_PORT: u16 = 4867;

impl Plugin for CommunicationsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        #[cfg(feature = "native")]
        app.add_plugin(ServerPlugin);

        app.add_state(ServerState::Closed)
            .add_state(ClientState::Closed)
            .add_plugin(ClientPlugin)
            .init_resource::<CommunicationResource>()
            .add_system(display_connection_ui.system());
    }
}

fn display_connection_ui(
    egui_context: ResMut<EguiContext>,
    mut communications: ResMut<CommunicationResource>,
    mut server_state: ResMut<State<ServerState>>,
    mut client_state: ResMut<State<ClientState>>,
    mut server_events: EventWriter<CloseServerEvent>,
) {
    egui::Window::new("Connection").show(egui_context.ctx(), |ui| {
        let mut is_server = false;
        let mut is_client = false;
        match &communications.state {
            CommunicationState::None => {}
            CommunicationState::Server { port: _ } => {
                is_server = true;
            }
            CommunicationState::Client { url: _ } => {
                is_client = true;
            }
        };

        ui.label(if is_client {
            "Is Client"
        } else if is_server {
            "Is Server"
        } else {
            "Isn't involved in the network"
        });

        if !communications.running {
            ui.horizontal(|ui| {
                #[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
                if ui.selectable_label(is_server, "Server").clicked() {
                    if is_server {
                        communications.state = CommunicationState::None;
                    } else {
                        communications.state = CommunicationState::Server { port: DEFAULT_PORT };
                    }
                }

                if ui.selectable_label(is_client, "Client").clicked() {
                    if is_client {
                        communications.state = CommunicationState::None;
                    } else {
                        communications.state = CommunicationState::Client {
                            url: format!("ws://localhost:{}", DEFAULT_PORT),
                        };
                    }
                }
            });
        }

        ui.horizontal(|ui| {
            if let CommunicationState::Server { port } = &communications.state {
                if !communications.running {
                    ui.label("Port:");
                    let mut port = port.to_string();
                    if ui.text_edit_singleline(&mut port).changed() {
                        if let Ok(i) = port.parse::<u16>() {
                            println!("Parsed {}", port);
                            communications.state = CommunicationState::Server { port: i };
                        } else {
                            println!("COULDNT PARSE {}", port);
                        }
                    }

                    if ui.button("Start Host").clicked() {
                        communications.running = true;
                        if server_state.push(ServerState::Open).is_ok() {
                            println!("Starting Host")
                        } else {
                            eprintln!("Failed to set host state");
                        }
                    }
                } else {
                    ui.label(format!("Host Running on Port {}", port));
                    if ui.button("Close Host").clicked() {
                        println!("Attempting to close server");
                        communications.running = false;
                        server_events.send(CloseServerEvent {});
                    }
                }
            }

            if let CommunicationState::Client { url } = &communications.state {
                if !communications.running {
                    ui.label("Server Address:");
                    let mut url = url.clone();
                    if ui.text_edit_singleline(&mut url).changed() {
                        communications.state = CommunicationState::Client { url };
                    }
                    if ui.button("Start Client").clicked() {
                        communications.running = true;
                        if client_state.push(ClientState::Open).is_ok() {
                            println!("Starting Client")
                        } else {
                            eprintln!("Failed to set client state")
                        }
                    }
                } else {
                    ui.label(format!("Connected to server at {}", url));
                }
            }
        })
    });
}
