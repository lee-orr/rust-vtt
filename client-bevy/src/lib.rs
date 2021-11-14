pub mod communications;
pub mod sdf_renderer;
mod camera;

use bevy::{PipelinedDefaultPlugins, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, prelude::*};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use communications::CommunicationsPlugin;
use sdf_renderer::{SdfPlugin, sdf_operation::{SDFBrush, SDFOperation, SDFShape}};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(CommunicationsPlugin)
        .add_plugin(SdfPlugin)
        .add_plugin(camera::CameraPlugin)
        .add_startup_system(setup.system())
        .add_system(ui)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}

fn ui(egui_context: ResMut<EguiContext>) {
    egui::Window::new("Hi").show(egui_context.ctx(), |ui| {
        ui.label("WORLD");
    });
}

fn setup(mut commands: Commands) {
    println!("Setting Up Brushes");
    commands.spawn()
        .insert(Transform::from_translation(Vec3::ZERO))
        .insert(GlobalTransform::default())
        .insert(SDFBrush { order: 0, shape: SDFShape::Box(1.,1.,1.), operation: SDFOperation::Union, blending: 0.});
        
    commands.spawn()
        .insert(Transform::from_translation(Vec3::new(2., 0., 0.)))
        .insert(GlobalTransform::default())
        .insert(SDFBrush { order: 1, shape: SDFShape::Sphere(2.), operation: SDFOperation::Intersection, blending: 1.});
}
