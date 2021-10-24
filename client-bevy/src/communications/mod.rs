use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

pub struct CommunicationsPlugin;

#[derive(Default)]
pub struct CommunicationResource {
    state: CommunicationState,
}

pub enum CommunicationState {
    None,
    Server { port: u16 },
    Client { url: String }
}

impl Default for CommunicationState {
    fn default() -> Self {
        CommunicationState::None
    }
}

const DEFAULT_PORT: u16 = 4867;

impl Plugin for CommunicationsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .init_resource::<CommunicationResource>()
            .add_system(display_connection_ui.system());
    }
}

fn display_connection_ui(egui_context: ResMut<EguiContext>, communications: ResMut<CommunicationResource>) {
    egui::Window::new("Connection")
        .show(egui_context.ctx(), |ui| {
                ui.label(match &communications.state {
                    CommunicationState::None => String::from("Not connected"),
                    CommunicationState::Server { port } => format!("Serving on port {}", port),
                    CommunicationState::Client { url } => format!("Connecting to server at {}", url),
                });
        });
}