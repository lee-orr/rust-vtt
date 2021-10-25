use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

pub mod shared;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;

use shared::*;
use server::*;

pub struct CommunicationsPlugin;

const DEFAULT_PORT: u16 = 4867;

impl Plugin for CommunicationsPlugin {
    fn build(&self, app: &mut AppBuilder) {    
        #[cfg(not(target_arch = "wasm32"))]
        app
            .add_plugin(ServerPlugin);
            
        app
            .init_resource::<CommunicationResource>()
            .add_system(display_connection_ui.system());
    }
}

fn display_connection_ui(
    egui_context: ResMut<EguiContext>,
    mut communications: ResMut<CommunicationResource>,
    mut app_state: ResMut<State<ServerState>>,
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
                #[cfg(not(target_arch = "wasm32"))]
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
                            url: format!("localhost:{}", DEFAULT_PORT),
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
                        app_state.push(ServerState::Open);
                        println!("Starting Host")
                    }
                } else {
                    ui.label(format!("Host Running on Port {}", port));
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
                        println!("Starting Client")
                    }
                } else {
                    ui.label(format!("Connected to server at {}", url));
                }
            }
        })
    });
}
