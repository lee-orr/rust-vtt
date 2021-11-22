use std::num::NonZeroU32;

use bevy::{
    core_pipeline::draw_3d_graph,
    math::Vec3,
    prelude::{Commands, FromWorld, GlobalTransform, Plugin, Query, Res, ResMut, With, World},
    render2::{
        render_graph::{Node, RenderGraph},
        render_resource::{BindGroup, BindGroupLayout, ComputePipeline, Sampler, TextureView},
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        RenderApp, RenderStage,
    },
};
use crevice::{std140::AsStd140, std430::AsStd430};
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferSize,
    BufferUsages, ComputePassDescriptor, Extent3d, FilterMode, ImageCopyTexture, ImageDataLayout,
    Origin3d, PipelineLayout, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, TextureDescriptor, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

use super::sdf_operation::{SDFObjectTree, SDFRootTransform};

pub struct SDFBakerPlugin;

impl Plugin for SDFBakerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SDFBakerSettings>();
        let settings = app.world.get_resource::<SDFBakerSettings>();
        let settings = if let Some(settings) = settings {
            println!("Settings are ready!");
            settings.clone()
        } else {
            SDFBakerSettings::default()
        };
        let render_app = app
            .sub_app(RenderApp)
            .insert_resource(settings)
            .init_resource::<SDFBakerPipelineDefinitions>()
            .init_resource::<SDFTextures>()
            .init_resource::<SDFBakedLayerOrigins>()
            .init_resource::<BakingGroupResource>()
            .add_system_to_stage(RenderStage::Prepare, setup_textures)
            .add_system_to_stage(RenderStage::Extract, extract_sdf_origin)
            .add_system_to_stage(RenderStage::Queue, queue_baking_group)
            .add_system_to_stage(RenderStage::Queue, bake_sdf_texture);

        let sdf_bake_pass_node = SDFBakePassNode::new(&mut render_app.world);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        let draw_3d_graph = graph.get_sub_graph_mut(draw_3d_graph::NAME);
        if let Some(draw_3d_graph) = draw_3d_graph {
            draw_3d_graph.add_node(SDFBakePassNode::NAME, sdf_bake_pass_node);
        }
    }
}

pub struct SDFBakerPipelineDefinitions {
    shader_module: ShaderModule,
    pipeline_layout: PipelineLayout,
    brush_layout: BindGroupLayout,
    texture_layout: BindGroupLayout,
    compute: ComputePipeline,
}

impl FromWorld for SDFBakerPipelineDefinitions {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let brush_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline BrushBind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
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
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
            ],
        });
        let texture_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Baking Texture Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let compute_pipeline_layout =
            render_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[&brush_layout, &texture_layout],
                push_constant_ranges: &[],
            });
        let shader_source = format!(
            "{}{}{}{}",
            include_str!("shaders/general/structs.wgsl"),
            include_str!("shaders/general/compute_bindings.wgsl"),
            include_str!("shaders/general/sdf_calculator.wgsl"),
            include_str!("shaders/compute/compute.wgsl")
        );
        let compute_shader_module = render_device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Compute Shader Module"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source.as_str())),
        });

        let compute = render_device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Baking Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: "cmp_main",
        });
        SDFBakerPipelineDefinitions {
            shader_module: compute_shader_module,
            pipeline_layout: compute_pipeline_layout,
            brush_layout,
            texture_layout,
            compute,
        }
    }
}

pub struct SDFBakedLayer {
    pub layer_size: (u32, u32, u32),
    pub world_size: Vec3,
}

#[derive(Clone, Copy, AsStd140)]
pub struct SDFBakerSettings {
    pub max_size: Vec3,
    pub layer_size: Vec3,
    pub num_layers: u32,
    pub layer_multiplier: u32,
}

#[derive(Clone, Default, Debug, Copy, AsStd140)]
pub struct SDFBakedLayerOrigins {
    pub origin: Vec3,
}

impl Default for SDFBakerSettings {
    fn default() -> Self {
        Self {
            max_size: Vec3::new(100., 50., 100.),
            layer_size: Vec3::new(128., 64., 128.),
            num_layers: 1,
            layer_multiplier: 2,
        }
    }
}

pub struct SDFBakeOrigin;

fn extract_sdf_origin(mut commands: Commands, query: Query<&GlobalTransform, With<SDFBakeOrigin>>) {
    let originTransform = query.get_single();
    let origins = SDFBakedLayerOrigins {
        origin: if let Ok(transform) = originTransform {
            transform.translation
        } else {
            Vec3::ZERO
        },
    };
    commands.insert_resource(origins);
}

#[derive(Default)]
pub struct SDFTextures {
    pub texture: Option<CachedTexture>,
    pub view: Option<TextureView>,
    pub sampler: Option<Sampler>,
    pub storage: Option<TextureView>,
}

fn setup_textures(
    settings: Res<SDFBakerSettings>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut textures: ResMut<SDFTextures>,
) {
    if textures.texture.is_none() {
        let layer_size = settings.layer_size;
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Baked SDF"),
                size: Extent3d {
                    depth_or_array_layers: (layer_size.z as u32) * settings.num_layers,
                    width: layer_size.x as u32,
                    height: layer_size.y as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            },
        );
        let view = texture.default_view.clone();
        let storage = texture.texture.create_view(&TextureViewDescriptor {
            label: Some("Baked SDF StorageDescriptor"),
            format: Some(TextureFormat::Rgba8Unorm),
            dimension: Some(TextureViewDimension::D3),
            aspect: wgpu::TextureAspect::All,
            ..Default::default()
        });
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("Baked SDF Sampler"),
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        textures.texture = Some(texture);
        textures.view = Some(view);
        textures.sampler = Some(sampler);
        textures.storage = Some(storage);
    }
}

fn bake_sdf_texture(
    settings: Res<SDFBakerSettings>,
    mut textures: ResMut<SDFTextures>,
    mut queue: ResMut<RenderQueue>,
    sdf_roots: Query<(&SDFRootTransform, &SDFObjectTree)>,
) {
}

pub fn queue_baking_group(
    mut commands: Commands,
    bake_settings: Res<SDFBakerSettings>,
    origins: Res<SDFBakedLayerOrigins>,
    textures: Res<SDFTextures>,
    render_device: Res<RenderDevice>,
    sdf_pipeline: Res<SDFBakerPipelineDefinitions>,
    mut baked_binding: ResMut<BakingGroupResource>,
) {
    if let (Some(storage)) = (&textures.storage) {
        let setting_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Bake Settings"),
            contents: bytemuck::cast_slice(&[bake_settings.as_std140()]),
            usage: BufferUsages::UNIFORM,
        });
        let origin_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Bake Origins"),
            contents: bytemuck::cast_slice(&[origins.as_std140()]),
            usage: BufferUsages::UNIFORM,
        });
        let brush_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bake Bind Group"),
            layout: &sdf_pipeline.texture_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&storage),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: setting_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: origin_buffer.as_entire_binding(),
                },
            ],
        });
        baked_binding.binding = Some(brush_bind_group.clone());
        commands.spawn().insert(BakingGroup {
            binding: brush_bind_group,
        });
    }
}

pub struct SDFBakePassNode {}

impl SDFBakePassNode {
    pub const NAME: &'static str = "SDF BAKE PASS";

    pub fn new(_world: &mut World) -> Self {
        Self {}
    }
}

pub struct BakingGroup {
    pub binding: BindGroup,
}

#[derive(Default)]
pub struct BakingGroupResource {
    binding: Option<BindGroup>,
}

#[derive(Default)]
pub struct BrushBindingGroupResource {
    pub binding: Option<BindGroup>,
}

impl Node for SDFBakePassNode {
    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        let pipeline = world
            .get_resource::<SDFBakerPipelineDefinitions>()
            .expect("Pipeline Should Exist");
        let brush_binding = world
            .get_resource::<BrushBindingGroupResource>()
            .expect("Binding Should Exist")
            .binding
            .clone()
            .unwrap();
        let baking_binding = world
            .get_resource::<BakingGroupResource>()
            .expect("Baking group should exist")
            .binding
            .clone()
            .unwrap();
        let settings = world
            .get_resource::<SDFBakerSettings>()
            .expect("Bake settings should exist");

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Compute Pass"),
            });
        pass.set_pipeline(&pipeline.compute);
        pass.set_bind_group(0, &brush_binding, &[0, 0]);
        pass.set_bind_group(1, &baking_binding, &[0, 0]);
        pass.dispatch(
            (settings.layer_size.x / 8.).ceil() as u32,
            (settings.layer_size.y / 8.).ceil() as u32,
            (settings.layer_size.z * (settings.num_layers as f32) / 8.).ceil() as u32,
        );
        Ok(())
    }
}
