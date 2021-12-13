use bevy::{
    math::Vec3,
    prelude::{Commands, Component, Entity, GlobalTransform, Plugin, Query, ResMut, With},
    render2::{RenderApp, RenderStage},
};
use crevice::std140::AsStd140;

pub struct SDFOriginPlugin;

impl Plugin for SDFOriginPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFOrigin>()
            .add_system_to_stage(RenderStage::Extract, extract_sdf_origin)
            .add_system_to_stage(RenderStage::Prepare, prepare_sdf_origin);
    }
}

#[derive(Clone, Debug, Copy, AsStd140)]
pub struct SDFOrigin {
    pub origin: Vec3,
}

impl Default for SDFOrigin {
    fn default() -> Self {
        Self {
            origin: Vec3::new(9999999., 9999999., 9999999.),
        }
    }
}

#[derive(Component)]
pub struct SDFOriginComponent;

fn extract_sdf_origin(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform), With<SDFOriginComponent>>,
) {
    for (entity, transform) in query.iter() {
        commands
            .get_or_spawn(entity)
            .insert(*transform)
            .insert(SDFOriginComponent);
    }
}

fn prepare_sdf_origin(
    query: Query<(Entity, &GlobalTransform), With<SDFOriginComponent>>,
    mut origin: ResMut<SDFOrigin>,
) {
    let origin_transform = query.get_single();
    let transform = match origin_transform {
        Ok((_, transform)) => transform.translation,
        Err(_) => Vec3::ZERO,
    };
    origin.origin = transform;
}
