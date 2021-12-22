use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroup, BindGroupLayout, TextureView},
        renderer::{RenderDevice},
        texture::{CachedTexture, TextureCache},
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};

use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Extent3d, FilterMode, SamplerBindingType, SamplerDescriptor, ShaderStages,
    TextureDescriptor, TextureFormat, TextureUsages,
};



pub struct SDFDepthBindingPlugin;

impl Plugin for SDFDepthBindingPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFDepthBindingLayout>()
            .add_system_to_stage(RenderStage::Prepare, prepare_depth_pass_texture);
    }
}

pub struct SDFDepthBindingLayout {
    pub layout: BindGroupLayout,
}

impl FromWorld for SDFDepthBindingLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline Depth Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        Self { layout }
    }
}

#[derive(Component)]
pub struct DepthBindingGroup {
    pub texture: CachedTexture,
    pub view: TextureView,
    pub binding: BindGroup,
}

const DEPTH_PASS_RATIO: u32 = 16;

fn prepare_depth_pass_texture(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedView)>,
    mut texture_cache: ResMut<TextureCache>,
    bind_layout: Res<SDFDepthBindingLayout>,
) {
    for (entity, view) in views.iter() {
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Depth Pass"),
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: view.width / DEPTH_PASS_RATIO as u32,
                    height: view.height / DEPTH_PASS_RATIO as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Depth32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("Depth Sampler"),
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let view = texture.default_view.clone();
        let binding = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Depth Pass Binding Group"),
            layout: &bind_layout.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        commands.entity(entity).insert(DepthBindingGroup {
            texture,
            view,
            binding,
        });
    }
}
