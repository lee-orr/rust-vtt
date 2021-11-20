use bevy::{
    math::Vec3,
    prelude::{
        App, BuildChildren, Changed, Commands, CoreStage, Entity, GlobalTransform, Or, Plugin,
        Query, Transform,
    },
};
use crevice::std140::AsStd140;

use super::sdf_operation::{process_sdf_node, SDFNode, SDFObject, SDFObjectTree};

pub struct SdfBlockMeshingPlugin;

impl Plugin for SdfBlockMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::Last, place_sdf_surface_blocks);
    }
}

const BLOCKS_PER_LEVEL: u32 = 2;

pub struct SDFBlock {
    scale: f32,
}

#[derive(Debug, AsStd140, Default, Clone)]
pub struct GpuSDFBlock {
    pub scale: f32,
    pub position: Vec3,
}

pub fn extract_gpu_blocks(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform, &SDFBlock)>,
) {
    for (entity, transform, block) in query.iter() {
        commands.get_or_spawn(entity).insert(GpuSDFBlock {
            scale: block.scale,
            position: transform.translation,
        });
    }
}

fn place_sdf_surface_blocks(
    mut commands: Commands,
    object_query: Query<
        (Entity, &SDFObject, &SDFObjectTree, &Transform),
        Or<(Changed<SDFObjectTree>, Changed<Transform>)>,
    >,
    node_query: Query<(Entity, &SDFNode, Option<&Transform>)>,
) {
    for (entity, object, tree, _transform) in object_query.iter() {
        let center = tree.tree[0].center;
        let radius = tree.tree[0].radius;
        let min_dist = radius / (BLOCKS_PER_LEVEL as f32);
        let step_size = min_dist * 2.;
        let _step_squared = step_size * step_size;
        let min = center - radius + step_size / 2.;
        for x in 0..BLOCKS_PER_LEVEL {
            for y in 0..BLOCKS_PER_LEVEL {
                for z in 0..BLOCKS_PER_LEVEL {
                    let point = Vec3::new(
                        x as f32 * step_size + min.x,
                        y as f32 * step_size + min.y,
                        z as f32 * step_size + min.z,
                    );
                    let val = process_sdf_node(&point, &object.root, &node_query);
                    if val <= min_dist && val >= -min_dist {
                        let child = commands
                            .spawn()
                            .insert(SDFBlock { scale: step_size })
                            .insert(Transform::from_translation(point))
                            .insert(GlobalTransform::default())
                            .id();
                        commands.entity(entity).insert_children(0, &[child]);
                    }
                }
            }
        }
    }
}
