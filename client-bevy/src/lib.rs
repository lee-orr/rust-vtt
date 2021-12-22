mod camera;
pub mod communications;
pub mod sdf_renderer;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::{options::WgpuOptions, render_resource::WgpuFeatures},
    DefaultPlugins,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use communications::CommunicationsPlugin;
use sdf_renderer::{
    sdf_operation::{SDFObjectAsset, SDFOperation},
    SdfPlugin,
};
use wasm_bindgen::prelude::*;

use crate::sdf_renderer::{
    sdf_lights::SDFPointLight,
    sdf_operation::{SDFNodeData, SDFShape},
};

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 1 })
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
        //.add_system(animate)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}

fn ui(egui_context: ResMut<EguiContext>) {
    egui::Window::new("Hi").show(egui_context.ctx(), |ui| {
        ui.label("WORLD");
    });
}

const NUM_BRUSHES: i32 = 1;
const UNOPTIMIZED_OBJECTS: bool = false;
const TEST_SCENE: bool = true;
const TEST_OP: SDFOperation = SDFOperation::Union;

// fn animate(mut query: Query<(&Handle<SDFObjectAsset>, &mut Transform)>, time: Res<Time>) {
//     for (_, mut transform) in query.iter_mut() {
//         transform.translation += Vec3::X
//             * time.delta().as_secs_f32()
//             * (if time.seconds_since_startup() as i32 % 2 == 0 {
//                 0.5
//             } else {
//                 -0.5
//             });
//     }
// }

fn setup(
    mut commands: Commands,
    mut sdf_objects: ResMut<Assets<SDFObjectAsset>>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
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
    } else if TEST_SCENE {
        //Light
        commands
            .spawn()
            .insert(Transform::from_translation(Vec3::new(0., 10., 0.)))
            .insert(GlobalTransform::default())
            .insert(SDFPointLight {
                distance: 30.,
                color: Color::Rgba {
                    red: 1.,
                    green: 1.,
                    blue: 1.,
                    alpha: 20.,
                },
            });

        // Ground
        let ground = SDFObjectAsset::new(vec![
            SDFNodeData::Operation(SDFOperation::Paint, 0.1, 1, 5),
            SDFNodeData::Operation(SDFOperation::Subtraction, 0.2, 2, 4),
            SDFNodeData::Transform(3, Transform::from_translation(Vec3::new(0., -5., 0.))),
            SDFNodeData::Primitive(SDFShape::Box(20., 5., 30.), Vec3::new(0.2, 0.5, 0.1)),
            SDFNodeData::Primitive(SDFShape::Sphere(2.), Vec3::new(0.6, 0.4, 0.2)),
            SDFNodeData::Primitive(
                SDFShape::Bezier(
                    Vec3::new(3., 0., -5.),
                    Vec3::new(4., 0., 1.),
                    Vec3::new(2., 0., 4.),
                    1.,
                ),
                Vec3::new(0.8, 0.6, 0.3),
            ),
        ]);
        let ground = sdf_objects.add(ground);
        commands
            .spawn()
            .insert(ground)
            .insert(Transform::from_translation(Vec3::ZERO))
            .insert(GlobalTransform::default());

        // CastleWalls
        let castle_walls = SDFObjectAsset::new(vec![
            SDFNodeData::Operation(SDFOperation::Union, 0.1, 1, 2),
            SDFNodeData::Operation(SDFOperation::Union, 0.1, 3, 4),
            SDFNodeData::Operation(SDFOperation::Union, 0.1, 5, 6),
            SDFNodeData::Transform(7, Transform::from_translation(Vec3::new(0., 0., -3.))),
            SDFNodeData::Transform(7, Transform::from_translation(Vec3::new(0., 0., 3.))),
            SDFNodeData::Transform(8, Transform::from_translation(Vec3::new(3., 0., 0.))),
            SDFNodeData::Transform(8, Transform::from_translation(Vec3::new(-3., 0., 0.))),
            SDFNodeData::Primitive(SDFShape::Box(3., 1., 0.1), Vec3::new(0.7, 0.6, 0.7)),
            SDFNodeData::Primitive(SDFShape::Box(0.1, 1., 3.), Vec3::new(0.7, 0.6, 0.7)),
        ]);
        let castle_walls = sdf_objects.add(castle_walls);
        commands
            .spawn()
            .insert(castle_walls)
            .insert(Transform::from_translation(Vec3::X * 6.))
            .insert(GlobalTransform::default());
    } else {
        commands
            .spawn()
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(sdf_object);
    }
}
