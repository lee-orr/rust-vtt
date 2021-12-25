use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroup, BindGroupLayout, Buffer, TextureView},
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};


use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, Extent3d, FilterMode,
    SamplerBindingType, SamplerDescriptor, ShaderStages, TextureDescriptor, TextureFormat,
    TextureSampleType, TextureUsages,
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
}

impl FromWorld for SDFOcttreeBindingLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let read_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline Depth Bind Group Layout"),
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
            label: Some("SDF Pipeline Depth Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::StorageTexture {
                        view_dimension: wgpu::TextureViewDimension::D3,
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba8Snorm,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        Self {
            read_layout,
            write_layout,
        }
    }
}

#[derive(Component)]
pub struct OcttreeBindingGroups {
    pub texture: CachedTexture,
    pub view: TextureView,
    pub read_binding: BindGroup,
    pub write_binding: BindGroup,
    pub dispatches: Buffer,
    pub tree_depth: u32,
}

const BLOCK_DIMENSION: u32 = 8;
const NUM_BLOCKS: u32 = 10000;
const TREE_DEPTH: u32 = 16;

fn prepare_octtree_binding(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
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
                    depth_or_array_layers: BLOCK_DIMENSION * NUM_BLOCKS,
                    width: BLOCK_DIMENSION,
                    height: BLOCK_DIMENSION,
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
        let tree = render_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Octree Buffer"),
            size: (NUM_BLOCKS as u64) * 24 + 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let dispatches = render_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Octree Dispatch Buffer"),
            size: (TREE_DEPTH as u64) * 12,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
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
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: dispatches.as_entire_binding(),
                },
            ],
        });
        commands.entity(entity).insert(OcttreeBindingGroups {
            texture,
            view,
            read_binding,
            write_binding,
            tree_depth: TREE_DEPTH,
            dispatches,
        });
    }
}
