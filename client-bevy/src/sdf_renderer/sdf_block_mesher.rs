use bevy::{
    math::Vec3,
    prelude::{
        App, Assets, BuildChildren, Changed, Commands, CoreStage, Entity, GlobalTransform, Plugin,
        Query, ResMut, Transform,
    },
    render2::{
        color::Color,
        mesh::{shape, Indices, Mesh},
    },
};
use crevice::std140::AsStd140;
use wgpu::PrimitiveTopology;

use super::sdf_operation::{process_sdf_node, SDFNode, SDFObject, SDFObjectDirty, SDFObjectTree};

pub struct SdfBlockMeshingPlugin;

impl Plugin for SdfBlockMeshingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::Last, place_sdf_surface_blocks);
    }
}

const blocks_per_level: u32 = 4;
const max_levels: u32 = 1;

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
    object_query: Query<(Entity, &SDFObject, &SDFObjectTree), Changed<SDFObjectTree>>,
    node_query: Query<(Entity, &SDFNode, Option<&Transform>)>,
) {
    for (entity, object, tree) in object_query.iter() {
        if tree.tree.len() < 0 {
            break;
        }
        let center = tree.tree[0].center;
        let radius = tree.tree[0].radius;
        let min = center - radius;
        let step_size = (radius * 2.) / (blocks_per_level as f32);
        let min_dist = step_size;
        for x in 0..blocks_per_level {
            for y in 0..blocks_per_level {
                for z in 0..blocks_per_level {
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
