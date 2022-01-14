mod camera;
pub mod communications;
mod map_construction;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::options::WgpuOptions,
    DefaultPlugins,
};
use bevy_egui::EguiPlugin;
use communications::CommunicationsPlugin;
use map_construction::MapConstructionPlugin;
use wasm_bindgen::prelude::*;
use wgpu::Features;

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let mut app = App::new();

    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(MapConstructionPlugin)
        .add_plugin(CommunicationsPlugin)
        .add_plugin(camera::CameraPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_system(setup);

    #[cfg(not(target_arch = "wasm32"))]
    {
        app.insert_resource(WgpuOptions {
            features: Features::POLYGON_MODE_LINE,
            ..Default::default()
        })
        .add_plugin(WireframePlugin);
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 100.,
            ..Default::default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0., 15., 30.)),
        ..Default::default()
    });
}
