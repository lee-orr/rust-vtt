pub mod communications;
mod camera;
mod meshing;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use communications::CommunicationsPlugin;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::build();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(CommunicationsPlugin)
        .add_plugin(camera::CameraPlugin)
        .add_plugin(meshing::MeshingPlugin);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_startup_system(setup.system())
        .add_system(ui.system())
        .run();
}

fn ui(egui_context: ResMut<EguiContext>) {
    egui::Window::new("Hi").show(egui_context.ctx(), |ui| {
        ui.label("WORLD");
    });
}

fn setup(
    mut commands: Commands,
) {
    // light
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}
