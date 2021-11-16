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

fn setup(mut commands: Commands) {
    println!("Setting Up Brushes");
    // let mut is_box = true;
    // for i in 0..NUM_BRUSHES {
    //     for j in 0..NUM_BRUSHES {
    //         let object = commands.spawn().id();
    //         let cube = commands
    //             .spawn()
    //             .insert(SDFNode {
    //                 object,
    //                 data: SDFNodeData::Primitive( if is_box { SDFShape::Box(1., 1., 1.) } else { SDFShape::Sphere(0.7)}),
    //             })
    //             .id();
    //         commands.entity(object)
    //             .insert(Transform::from_translation(Vec3::new(
    //                 i as f32 * 4.,
    //                 0.,
    //                 -4. * (j as f32),
    //             )))
    //             .insert(GlobalTransform::default())
    //             .insert(SDFObject {
    //                 root: cube,
    //             });
    //         is_box = !is_box;
    //     }
    // }
    let object = commands.spawn().id();
    let cube = commands
        .spawn()
        .insert(SDFNode {
            object,
            data: SDFNodeData::Primitive(SDFShape::Box(0.2, 0.2, 0.2)),
        })
        .id();
    let cube_transform = commands.spawn().insert(SDFNode {
        object,
        data: SDFNodeData::Transform(cube)
    }).insert(Transform::default()).id();
    let sphere = commands
        .spawn()
        .insert(SDFNode {
            object,
            data: SDFNodeData::Primitive(SDFShape::Sphere(2.)),
        })
        .id();
    let sphere_transform = commands.spawn().insert(SDFNode {
        object,
        data: SDFNodeData::Transform(sphere)
    }).insert(Transform::from_translation(Vec3::new(2., 0., 0.))).id();
    let op = commands.spawn().insert(SDFNode {
        object,
        data: SDFNodeData::Operation(SDFOperation::Union, 0., cube_transform, sphere_transform)
    }).id();
    commands.entity(object)
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(SDFObject {
            root: op,
        });
}
