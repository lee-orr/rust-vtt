mod camera;
pub mod communications;
pub mod sdf_renderer;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    DefaultPlugins, pbr::wireframe::{Wireframe, WireframePlugin},
    render::{options::WgpuOptions, render_resource::WgpuFeatures},
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use communications::CommunicationsPlugin;
use sdf_renderer::{
    sdf_operation::{SDFObjectAsset, SDFOperation},
    SdfPlugin,
};
use wasm_bindgen::prelude::*;

use crate::sdf_renderer::sdf_lights::SDFPointLight;

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();
    app
        .insert_resource(Msaa { samples: 1 })
        .insert_resource(WgpuOptions {
            features: WgpuFeatures::POLYGON_MODE_LINE,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
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

const NUM_BRUSHES: i32 = 2;
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

fn setup(
    mut commands: Commands,
    mut sdf_objects: ResMut<Assets<SDFObjectAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Setting Up Brushes");
    let sdf_object = SDFObjectAsset::test_object(TEST_OP, 0.2);
    let sdf_object = sdf_objects.add(sdf_object);
    commands
        .spawn()
        .insert(Transform::from_translation(Vec3::new(0., 4., 0.)))
        .insert(GlobalTransform::default())
        .insert(SDFPointLight {
            distance: 30.,
            color: Color::Rgba {
                red: 1.,
                green: 1.,
                blue: 1.,
                alpha: 10.,
            },
        });
    commands
        .spawn()
        .insert(Transform::from_translation(Vec3::new(6., 4., 0.)))
        .insert(GlobalTransform::default())
        .insert(SDFPointLight {
            distance: 30.,
            color: Color::Rgba {
                red: 0.5,
                green: 1.,
                blue: 0.5,
                alpha: 5.,
            },
        });
    let cube = meshes.add(Mesh::from(shape::UVSphere {radius: 1., sectors: 6, stacks: 6 }));
    let material = materials.add(StandardMaterial { base_color: Color::BLUE, unlit: true, ..Default::default()});
    if UNOPTIMIZED_OBJECTS {
        for i in 0..NUM_BRUSHES {
            for j in 0..NUM_BRUSHES {
                commands
                    .spawn()
                    .insert_bundle(PbrBundle {
                        mesh: cube.clone(),
                        material: material.clone(),
                        ..Default::default()
                    })
                    .insert(Wireframe)
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
