use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{BindGroup, BindGroupLayout, Buffer, DynamicUniformVec},
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
};
use crevice::std140::AsStd140;
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, BufferUsages, ShaderStages,
};

use super::sdf_operation::{
    GpuSDFNode, SDFObjectAsset, SDFObjectCount, SDFRootTransform, Std140GpuSDFNode, TRANSFORM_WARP,
};

pub struct SDFBrushBindingPlugin;

impl Plugin for SDFBrushBindingPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFBrushBindingLayout>()
            .init_resource::<BrushUniforms>()
            .add_system_to_stage(RenderStage::Prepare, prepare_brush_uniforms)
            .add_system_to_stage(RenderStage::Queue, queue_brush_bindings);
    }
}

pub struct SDFBrushBindingLayout {
    pub layout: BindGroupLayout,
}

impl FromWorld for SDFBrushBindingLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline Brush Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
            ],
        });
        Self { layout }
    }
}

#[derive(Default)]
pub struct BrushUniforms {
    pub brushes: Option<Buffer>,
}

#[derive(Component)]
pub struct BrushBindingGroup {
    pub binding: BindGroup,
}

fn prepare_brush_uniforms(
    mut brush_uniforms: ResMut<BrushUniforms>,
    objects: Query<(&Handle<SDFObjectAsset>, &SDFRootTransform, Entity)>,
    render_device: Res<RenderDevice>,
    sdf_objects: Res<RenderAssets<SDFObjectAsset>>,
) {
    let mut objects = objects.iter().collect::<Vec<_>>();
    objects.sort_by(|a, b| a.2.cmp(&b.2));

    let object_count = objects.len();
    let mut index_so_far = object_count;
    let mut brush_vec: Vec<GpuSDFNode> = Vec::new();
    for (tree, transform, _) in &objects {
        if let Some(tree) = sdf_objects.get(tree) {
            let num_nodes = tree.tree.len();
            if num_nodes > 0 {
                let root = &tree.tree[0];
                let child = (index_so_far - brush_vec.len()) as i32;
                let transform = GpuSDFNode {
                    node_type: TRANSFORM_WARP,
                    child_a: child,
                    center: root.center - transform.translation,
                    radius: root.radius * transform.scale.max_element(),
                    params: transform.matrix,
                    ..Default::default()
                };
                brush_vec.push(transform);
                index_so_far += num_nodes;
            } else {
                brush_vec.push(GpuSDFNode::default());
            }
        } else {
            brush_vec.push(GpuSDFNode::default());
        }
    }
    for (tree, _, _) in &objects {
        if let Some(tree) = sdf_objects.get(tree) {
            for node in &tree.tree {
                brush_vec.push(node.clone());
            }
        }
    }
    println!("Brushes {:?}", &brush_vec);

    let mut brushes: Vec<Std140GpuSDFNode> = brush_vec.iter().map(|val| val.as_std140()).collect();

    if brushes.is_empty() {
        brushes.push(GpuSDFNode::default().as_std140());
    }

    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Brush Buffer"),
        contents: bytemuck::cast_slice(brushes.as_slice()),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    brush_uniforms.brushes = Some(buffer);
}

pub fn queue_brush_bindings(
    mut commands: Commands,
    buffers: Res<BrushUniforms>,
    object_count: Res<SDFObjectCount>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    bind_layout: Res<SDFBrushBindingLayout>,
) {
    let mut count = DynamicUniformVec::<SDFObjectCount>::default();
    count.clear();
    count.push(*object_count);
    count.write_buffer(&render_device, &render_queue);
    if let Some(brushes) = &buffers.brushes {
        let brush_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Brush Bind Group"),
            layout: &bind_layout.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: brushes.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: count.binding().unwrap(),
                },
            ],
        });
        let binding = BrushBindingGroup {
            binding: brush_bind_group,
        };
        commands.spawn().insert(binding);
    }
}
