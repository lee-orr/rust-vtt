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
    sdf_operation::{SDFOperation, SDFShape},
    SdfPlugin,
};
use wasm_bindgen::prelude::*;

use crate::sdf_renderer::sdf_operation::{SDFNode, SDFNodeData, SDFObject};

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
const OPTIMIZED_OBJECTS: bool = false;
const TEST_OP: SDFOperation = SDFOperation::Union;

fn spawn_optimized_hierarchy(
    commands: &mut Commands,
    object: &Entity,
    num_brushes: u32,
) -> Option<Entity> {
    if num_brushes == 0 {
        return None;
    }
    if num_brushes == 1 {
        let cube = commands
            .spawn()
            .insert(SDFNode {
                object: *object,
                data: SDFNodeData::Primitive(SDFShape::Box(1., 1., 1.)),
            })
            .id();
        return Some(cube);
    }
    if num_brushes % 4 == 0 {
        let num_children = num_brushes / 4;
        let child_1 = spawn_optimized_hierarchy(commands, object, num_children);
        let child_2 = spawn_optimized_hierarchy(commands, object, num_children);
        let child_3 = spawn_optimized_hierarchy(commands, object, num_children);
        let child_4 = spawn_optimized_hierarchy(commands, object, num_children);
        let level = (num_brushes / 2) as f32;
        if let (Some(child_1), Some(child_2), Some(_child_3), Some(_child_4)) =
            (child_1, child_2, child_3, child_4)
        {
            let transform_1 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_1),
                })
                .insert(Transform::from_translation(Vec3::new(level, 0., level)))
                .id();
            let transform_2 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_2),
                })
                .insert(Transform::from_translation(Vec3::new(level, 0., -level)))
                .id();
            let transform_3 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_2),
                })
                .insert(Transform::from_translation(Vec3::new(-level, 0., level)))
                .id();
            let transform_4 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_2),
                })
                .insert(Transform::from_translation(Vec3::new(-level, 0., -level)))
                .id();
            let op1 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Operation(SDFOperation::Union, 0., transform_1, transform_2),
                })
                .id();
            let op2 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Operation(SDFOperation::Union, 0., transform_3, transform_4),
                })
                .id();
            let op3 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Operation(SDFOperation::Union, 0., op1, op2),
                })
                .id();
            return Some(op3);
        }
        return None;
    }
    if num_brushes % 3 == 0 {
        let third = num_brushes / 3;
        let child_1 = spawn_optimized_hierarchy(commands, object, third * 2);
        let child_2 = spawn_optimized_hierarchy(commands, object, third);
        if let (Some(child_1), Some(child_2)) = (child_1, child_2) {
            let transform_1 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_1),
                })
                .insert(Transform::from_translation(Vec3::Z * 2.))
                .id();
            let transform_2 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_2),
                })
                .insert(Transform::from_translation(Vec3::Z * -2.))
                .id();
            let op = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Operation(SDFOperation::Union, 0., transform_1, transform_2),
                })
                .id();
            return Some(op);
        }
        return None;
    }
    if num_brushes % 2 == 0 {
        let child_1 = spawn_optimized_hierarchy(commands, object, num_brushes / 2);
        let child_2 = spawn_optimized_hierarchy(commands, object, num_brushes / 2);
        if let (Some(child_1), Some(child_2)) = (child_1, child_2) {
            let transform_1 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_1),
                })
                .insert(Transform::from_translation(Vec3::X * 2.))
                .id();
            let transform_2 = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Transform(child_2),
                })
                .insert(Transform::from_translation(Vec3::X * -2.))
                .id();
            let op = commands
                .spawn()
                .insert(SDFNode {
                    object: *object,
                    data: SDFNodeData::Operation(SDFOperation::Union, 0., transform_1, transform_2),
                })
                .id();
            return Some(op);
        }
        return None;
    }
    None
}

fn animate(mut query: Query<(&SDFObject, &mut Transform)>, time: Res<Time>) {
    for (_, mut transform) in query.iter_mut() {
        transform.translation += Vec3::X * time.delta().as_secs_f32() * (if time.seconds_since_startup() as i32 % 2 == 0 { 0.5 } else { -0.5 });
    }
}

fn setup(mut commands: Commands) {
    println!("Setting Up Brushes");
    if UNOPTIMIZED_OBJECTS {
        for i in 0..NUM_BRUSHES {
            for j in 0..NUM_BRUSHES {
                let object = commands.spawn().id();
                let cube = commands
                    .spawn()
                    .insert(SDFNode {
                        object,
                        data: SDFNodeData::Primitive(SDFShape::Box(1., 1., 1.)),
                    })
                    .id();
                commands
                    .entity(object)
                    .insert(Transform::from_translation(Vec3::new(
                        i as f32 * 4.,
                        0.,
                        -4. * (j as f32),
                    )))
                    .insert(GlobalTransform::default())
                    .insert(SDFObject { root: cube });
            }
        }
    } else if OPTIMIZED_OBJECTS {
        let object = commands.spawn().id();
        let root =
            spawn_optimized_hierarchy(&mut commands, &object, (NUM_BRUSHES * NUM_BRUSHES) as u32);

        if let Some(root) = root {
            commands
                .entity(object)
                .insert(Transform::from_translation(Vec3::ZERO))
                .insert(GlobalTransform::default())
                .insert(SDFObject { root });
        }
    } else {
        let object = commands.spawn().id();
        let cube = commands
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Box(0.2, 0.2, 0.2)),
            })
            .id();
        let cube_transform = commands
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Transform(cube),
            })
            .insert(Transform::default())
            .id();
        let sphere = commands
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Sphere(2.)),
            })
            .id();
        let sphere_transform = commands
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Transform(sphere),
            })
            .insert(Transform::from_translation(Vec3::new(2., 0., 0.)))
            .id();
        let op = commands
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Operation(TEST_OP, 0., cube_transform, sphere_transform),
            })
            .id();
        commands
            .entity(object)
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(SDFObject { root: op });
    }
}
