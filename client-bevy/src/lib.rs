mod camera;
pub mod communications;
pub mod sdf_renderer;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    PipelinedDefaultPlugins,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use communications::CommunicationsPlugin;
use sdf_renderer::{
    sdf_operation::{SDFBrush, SDFOperation, SDFShape},
    SdfPlugin,
};
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

const NUM_BRUSHES: i32 = 4;

fn setup(mut commands: Commands) {
    println!("Setting Up Brushes");
    for i in 0..NUM_BRUSHES {
        for j in 0..NUM_BRUSHES {
            commands
                .spawn()
                .insert(Transform::from_translation(Vec3::new(
                    i as f32,
                    0.,
                    -1. * (j as f32),
                )))
                .insert(GlobalTransform::default())
                .insert(SDFBrush {
                    order: 0,
                    shape: SDFShape::Box(0.2, 0.2, 0.2),
                    operation: SDFOperation::Union,
                    blending: 0.,
                });
        }
    }
    /*     commands
            .spawn()
            .insert(Transform::from_translation(Vec3::ZERO))
            .insert(GlobalTransform::default())
            .insert(SDFBrush {
                order: 0,
                shape: SDFShape::Box(1., 1., 1.),
                operation: SDFOperation::Union,
                blending: 0.,
            });
        commands
            .spawn()
            .insert(Transform::from_translation(Vec3::new(0., 0., -50.)))
            .insert(GlobalTransform::default())
            .insert(SDFBrush {
                order: 2,
                shape: SDFShape::Box(1., 1., 1.),
                operation: SDFOperation::Union,
                blending: 0.,
            });
        commands
            .spawn()
            .insert(Transform::from_translation(Vec3::new(0., 0., -95.)))
            .insert(GlobalTransform::default())
            .insert(SDFBrush {
                order: 2,
                shape: SDFShape::Box(1., 1., 1.),
                operation: SDFOperation::Union,
                blending: 0.,
            });

        commands
            .spawn()
            .insert(Transform::from_translation(Vec3::new(2., 0., 0.)))
            .insert(GlobalTransform::default())
            .insert(SDFBrush {
                order: 1,
                shape: SDFShape::Sphere(2.),
                operation: SDFOperation::Intersection,
                blending: 1.,
            });
    */
}
