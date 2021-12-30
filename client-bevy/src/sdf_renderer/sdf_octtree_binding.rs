use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroup, BindGroupLayout, Buffer, DynamicUniformVec, TextureView},
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};

use crevice::std140::AsStd140;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferSize, BufferUsages, Extent3d,
    FilterMode, SamplerBindingType, SamplerDescriptor, ShaderStages, TextureDescriptor,
    TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, util::BufferInitDescriptor,
};

pub struct SDFOcttreeBindingPlugin;

impl Plugin for SDFOcttreeBindingPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFOcttreeBindingLayout>()
            .add_system_to_stage(RenderStage::Prepare, prepare_octtree_binding);
    }
}

pub struct SDFOcttreeBindingLayout {
    pub read_layout: BindGroupLayout,
    pub write_layout: BindGroupLayout,
    pub dispatch_layout: BindGroupLayout
}

impl FromWorld for SDFOcttreeBindingLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let read_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Octree Read Layout"),
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
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let write_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Octtree Write Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        view_dimension: wgpu::TextureViewDimension::D3,
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Snorm,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let dispatch_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Octree Read Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(12),
                    },
                    count: None,
                },
            ],
        });
        Self {
            read_layout,
            write_layout,
            dispatch_layout,
        }
    }
}

#[derive(Component)]
pub struct OcttreeBindingGroups {
    pub texture: CachedTexture,
    pub view: TextureView,
    pub read_binding: BindGroup,
    pub write_binding: BindGroup,
    pub tree_depth: u32,
    pub block_dimension: u32,
    pub dispatch_1_binding: BindGroup,
    pub dispatch_2_binding: BindGroup,
    pub dispatch1: Buffer,
    pub dispatch2: Buffer
}

const BLOCK_DIMENSION: u32 = 5;
const BLOCKS_PER_DIMENSION : u32 = 100;
const NUM_BLOCKS: u32 = BLOCKS_PER_DIMENSION * BLOCKS_PER_DIMENSION * BLOCKS_PER_DIMENSION;
pub const TREE_DEPTH: u32 = 16;

#[derive(AsStd140)]
struct TreeBakeSettings {
    pub num_blocks: u32,
    pub blocks_per_dimension: u32,
    pub current_layer: i32,
}
#[derive(AsStd140)]
struct DynamicDispatch {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}
fn prepare_octtree_binding(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView)>,
    mut texture_cache: ResMut<TextureCache>,
    bind_layout: Res<SDFOcttreeBindingLayout>,
) {
    for (entity, _view) in views.iter() {
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Octtree Pass"),
                size: Extent3d {
                    depth_or_array_layers: BLOCK_DIMENSION * BLOCKS_PER_DIMENSION,
                    width: BLOCK_DIMENSION * BLOCKS_PER_DIMENSION,
                    height: BLOCK_DIMENSION * BLOCKS_PER_DIMENSION,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: TextureFormat::Rgba8Snorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            },
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("Octtree Sampler"),
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let view = texture.default_view.clone();
        let storage = texture.texture.create_view(&TextureViewDescriptor {
            label: Some("Baked SDF StorageDescriptor"),
            format: Some(TextureFormat::Rgba8Snorm),
            dimension: Some(TextureViewDimension::D3),
            aspect: wgpu::TextureAspect::All,
            ..Default::default()
        });
        let tree = render_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Octree Buffer"),
            size: (NUM_BLOCKS as u64) * 48 + 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let start_points = render_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Octree Start Point Buffer"),
            size: (TREE_DEPTH as u64) * 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let mut count = DynamicUniformVec::<TreeBakeSettings>::default();
        count.clear();
        for i in 0..TREE_DEPTH {
            count.push(TreeBakeSettings {
                num_blocks: NUM_BLOCKS,
                blocks_per_dimension: BLOCKS_PER_DIMENSION,
                current_layer: i as i32,
            });
        }
        count.write_buffer(&render_device, &render_queue);

        
        let mut dispatch1 = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("dispatch1 buffer"),
            contents: bytemuck::cast_slice(&[DynamicDispatch { x: 1, y: 0, z: 0}.as_std140()]),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
        });

        let mut dispatch2 = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("dispatch1 buffer"),
            contents: bytemuck::cast_slice(&[DynamicDispatch { x: 1, y: 0, z: 0}.as_std140()]),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
        });

        let read_binding = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Octtree Read Binding Group"),
            layout: &bind_layout.read_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: tree.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        let write_binding = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Octtree Write Binding Group"),
            layout: &bind_layout.write_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: tree.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&storage),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: count.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: start_points.as_entire_binding(),
                },
            ],
        });
        let dispatch_1_binding = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Octtree Dispatch 1 Binding Group"),
            layout: &bind_layout.dispatch_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: dispatch1.as_entire_binding(),
                },
            ],
        });
        let dispatch_2_binding = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Octtree Dispatch 2 Binding Group"),
            layout: &bind_layout.dispatch_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: dispatch2.as_entire_binding(),
                },
            ],
        });
        
        commands.entity(entity).insert(OcttreeBindingGroups {
            texture,
            view,
            read_binding,
            write_binding,
            tree_depth: TREE_DEPTH,
            block_dimension: BLOCK_DIMENSION,
            dispatch_1_binding,
            dispatch_2_binding,
            dispatch1: dispatch1,
            dispatch2: dispatch2
        });
    }
}
