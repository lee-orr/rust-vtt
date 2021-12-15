mod camera;
pub mod communications;
pub mod sdf_renderer;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    DefaultPlugins,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use communications::CommunicationsPlugin;
use sdf_renderer::{
    sdf_operation::{SDFObjectAsset, SDFOperation},
    SdfPlugin,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(CommunicationsPlugin)
        .add_plugin(SdfPlugin)
        .add_plugin(camera::CameraPlugin)
        .add_startup_system(setup.system())
        .add_system(ui)
        .add_system(animate)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}

fn ui(egui_context: ResMut<EguiContext>) {
    egui::Window::new("Hi").show(egui_context.ctx(), |ui| {
        ui.label("WORLD");
    });
}

const NUM_BRUSHES: i32 = 50;
const UNOPTIMIZED_OBJECTS: bool = true;
const TEST_OP: SDFOperation = SDFOperation::Union;

fn animate(mut query: Query<(&Handle<SDFObjectAsset>, &mut Transform)>, time: Res<Time>) {
    for (_, mut transform) in query.iter_mut() {
        transform.translation += Vec3::X
            * time.delta().as_secs_f32()
            * (if time.seconds_since_startup() as i32 % 2 == 0 {
                0.5
            } else {
                -0.5
            });
    }
}

fn setup(mut commands: Commands, mut sdf_objects: ResMut<Assets<SDFObjectAsset>>) {
    println!("Setting Up Brushes");
    let sdf_object = SDFObjectAsset::test_object(TEST_OP, 0.5);
    let sdf_object = sdf_objects.add(sdf_object);
    if UNOPTIMIZED_OBJECTS {
        for i in 0..NUM_BRUSHES {
            for j in 0..NUM_BRUSHES {
                commands
                    .spawn()
                    .insert(sdf_object.clone())
                    .insert(Transform::from_translation(Vec3::new(
                        i as f32 * 4.,
                        0.,
                        -4. * (j as f32),
                    )))
                    .insert(GlobalTransform::default());
            }
        }
    } else {
        commands
            .spawn()
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(sdf_object);
    }
}
