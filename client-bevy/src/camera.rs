use bevy::{
    input::Input,
    math::prelude::*,
    prelude::{
        BuildChildren, Commands, Component, GlobalTransform, KeyCode, Plugin, Query, Res, Time,
        Transform, With,
    },
    render::camera::{PerspectiveCameraBundle, PerspectiveProjection},
};

use crate::sdf_renderer::sdf_origin::SDFOriginComponent;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup)
            .add_system(move_camera_focus)
            .add_system(move_camera);
    }
}

fn setup(mut commands: Commands) {
    let parent = commands
        .spawn()
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(CameraFocus)
        .id();

    let camera = commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(SDFOriginComponent)
        .insert(CameraRadius { radius: 15.0 })
        .insert(CameraHeight { height: 15.0 })
        .id();

    commands.entity(parent).push_children(&[camera]);
}

#[derive(Component)]
pub struct CameraFocus;

#[derive(Component)]
pub struct CameraRadius {
    pub radius: f32,
}

#[derive(Component)]
pub struct CameraHeight {
    pub height: f32,
}

fn move_camera_focus(
    mut focus_query: Query<&mut Transform, With<CameraFocus>>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut movement = Vec3::ZERO;
    let mut rotation: f32 = 0.0;
    if keys.pressed(KeyCode::W) {
        movement = Vec3::new(0., 0., 1.);
    } else if keys.pressed(KeyCode::S) {
        movement = Vec3::new(0., 0., -1.);
    } else if keys.pressed(KeyCode::A) {
        movement = Vec3::new(1., 0., 0.);
    } else if keys.pressed(KeyCode::D) {
        movement = Vec3::new(-1., 0., 0.);
    }
    if keys.pressed(KeyCode::E) {
        rotation += 1.;
    } else if keys.pressed(KeyCode::Q) {
        rotation -= 1.;
    }
    rotation = rotation * std::f32::consts::FRAC_1_PI * time.delta_seconds();
    movement = movement.normalize_or_zero() * time.delta_seconds();
    for mut transform in focus_query.iter_mut() {
        let offset = transform.local_x() * movement.x
            + transform.local_y() * movement.y
            + transform.local_z() * movement.z;
        transform.translation += offset;
        transform.rotate(Quat::from_rotation_y(rotation));
    }
}

fn move_camera(
    mut camera_query: Query<(&mut Transform, &mut CameraRadius, &mut CameraHeight)>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut height_change: f32 = 0.;
    let mut zoom_change: f32 = 0.;

    if keys.pressed(KeyCode::LShift) {
        height_change += 1.;
        zoom_change += 1.;
    } else if keys.pressed(KeyCode::LControl) {
        height_change -= 1.;
        zoom_change -= 1.;
    }

    height_change *= time.delta_seconds();
    zoom_change *= time.delta_seconds();

    for (mut transform, mut radius, mut height) in camera_query.iter_mut() {
        height.height += height_change;
        radius.radius += zoom_change;

        transform.translation = Vec3::new(0., height.height, -1. * radius.radius);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
