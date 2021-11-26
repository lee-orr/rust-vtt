

use std::collections::HashMap;

use bevy::{core_pipeline::draw_3d_graph, math::Vec3, prelude::{
        Changed, Commands, Entity, FromWorld, GlobalTransform, Plugin, Query, Res, ResMut, With, World,
    }, render2::{
        render_graph::{Node, RenderGraph},
        render_resource::{BindGroup, BindGroupLayout, ComputePipeline, Sampler, TextureView},
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        RenderApp, RenderStage,
    }};
use crevice::std140::AsStd140;
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferSize,
    BufferUsages, ComputePassDescriptor, Extent3d, FilterMode, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureDescriptor,
    TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use super::sdf_operation::{
    SDFGlobalNodeBounds, SDFObjectTree, SDFRootTransform,
};

pub struct SDFBakerPlugin;

impl Plugin for SDFBakerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SDFBakerSettings>();
        let settings = app.world.get_resource::<SDFBakerSettings>();
        let settings = if let Some(settings) = settings {
            println!("Settings are ready!");
            *settings
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
            .init_resource::<ReBakeSDFResource>()
            .init_resource::<SDFZones>()
            .add_system_to_stage(RenderStage::Prepare, setup_textures)
            .add_system_to_stage(RenderStage::Extract, extract_sdf_origin)
            .add_system_to_stage(RenderStage::Extract, extract_rebuild)
            .add_system_to_stage(RenderStage::Prepare, prepare_sdf_origin)
            .add_system_to_stage(RenderStage::Prepare, prepare_zones)
            .add_system_to_stage(RenderStage::Queue, prepare_bake)
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
    zone_layout: BindGroupLayout,
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
                        format: TextureFormat::R8Unorm,
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
        let zone_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Zone Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
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
                    visibility: ShaderStages::COMPUTE,
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
                bind_group_layouts: &[&brush_layout, &texture_layout, &zone_layout],
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
            texture_layout,
            zone_layout,
            compute,
        }
    }
}

#[derive(Clone, Copy, AsStd140)]
pub struct SDFBakerSettings {
    pub max_size: Vec3,
    pub layer_size: Vec3,
    pub num_layers: u32,
    pub layer_multiplier: u32,
}

#[derive(Clone, Debug, Copy, AsStd140)]
pub struct SDFBakedLayerOrigins {
    pub origin: Vec3,
}

#[derive(Clone, Debug, Copy, AsStd140, Default)]
pub struct SDFZoneDefinitions {
    pub min: Vec3,
    pub max: Vec3,
    pub first_object: i32,
    pub final_object: i32,
}

impl Default for SDFBakedLayerOrigins {
    fn default() -> Self {
        Self {
            origin: Vec3::new(9999999., 9999999., 9999999.),
        }
    }
}

impl Default for SDFBakerSettings {
    fn default() -> Self {
        Self {
            max_size: Vec3::new(100., 25., 100.),
            layer_size: Vec3::new(512., 64., 512.),
            num_layers: 1,
            layer_multiplier: 2,
        }
    }
}

pub struct SDFBakeOrigin;

pub struct ReBakeSDF;

#[derive(Default)]
pub struct ReBakeSDFResource {
    pub rebake: bool,
}

#[derive(Default)]
pub struct SDFZones {
    zone_group: Option<BindGroup>,
}

fn extract_sdf_origin(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform), With<SDFBakeOrigin>>,
    _settings: Res<SDFBakerSettings>,
) {
    for (entity, transform) in query.iter() {
        commands
            .get_or_spawn(entity)
            .insert(*transform)
            .insert(SDFBakeOrigin);
    }
}

fn extract_rebuild(
    mut commands: Commands,
    query: Query<(Entity, &SDFGlobalNodeBounds), Changed<SDFObjectTree>>,
) {
    let exists = query.get_single().is_ok();
    if exists {
        commands.spawn().insert(ReBakeSDF);
    }
}

const ZONES_PER_DIMENSION: i32 = 8;

#[derive(AsStd140)]
struct NumZones {
    num_zones: i32,
    zone_radius: f32,
    zone_size: Vec3,
    zone_origin: Vec3,
    zones_per_dimension: i32,
}

fn prepare_zones(
    query: Query<(Entity, &SDFGlobalNodeBounds, &SDFObjectTree)>,
    render_device: Res<RenderDevice>,
    settings: Res<SDFBakerSettings>,
    origin: Res<SDFBakedLayerOrigins>,
    mut zones: ResMut<SDFZones>,
    sdf_pipeline: Res<SDFBakerPipelineDefinitions>,
) {
    let mut objects = query.iter().collect::<Vec<_>>();
    objects.sort_by(|a, b| a.0.cmp(&b.0));
    let mut zone_objects: Vec<i32> = Vec::new();
    let mut active_zones: Vec<SDFZoneDefinitions> = Vec::new();
    let mut zone_hash : HashMap<(u32, u32, u32), Vec<i32>> = HashMap::new();
    let zone_size = (settings.max_size / (ZONES_PER_DIMENSION as f32)).abs();
    let zone_half_size = zone_size / 2.;
    let zone_radius = zone_half_size.length();
    let bounds_min = origin.origin - (settings.max_size / 2.);
    let _bounds_max = origin.origin + (settings.max_size / 2.);
    let voxel_size = (settings.max_size / settings.layer_size).max_element();
    let effective_radius = zone_radius * 2. + 8. * voxel_size;

    for (obj, (_, bounds, _)) in objects.iter().enumerate() {
        let zone_bound_radius = bounds.radius * 2. + effective_radius;
        let min_zone_bound = bounds.center - zone_bound_radius;
        let max_zone_bound = bounds.center + zone_bound_radius;
        let min_zone_bound = ((min_zone_bound - bounds_min) / zone_size).floor();
        let max_zone_bound = ((max_zone_bound - bounds_min) / zone_size).floor();
        for x in (min_zone_bound.x as i32)..(max_zone_bound.x as i32) {
            if x > ZONES_PER_DIMENSION { break; }
            for y in (min_zone_bound.y as i32)..(max_zone_bound.y as i32) {
                if y > ZONES_PER_DIMENSION { break; }
                for z in (min_zone_bound.z as i32)..(max_zone_bound.z as i32) {
                    if z > ZONES_PER_DIMENSION { break; }
                    let key = (x as u32, y as u32, z as u32);
                    if !zone_hash.contains_key(&key) {
                        zone_hash.insert(*&key, Vec::new());
                    }
                    let mut vec = zone_hash.get_mut(&key);
                    if let Some(mut vec) = vec {
                        vec.push(obj as i32);
                    }
                }
            }
        }
    }

    for ((x,y,z), mut vec) in zone_hash.into_iter() {
        let offset = Vec3::new(x as f32, y as f32, z as f32);
        let position = offset * zone_size
            + bounds_min
            + zone_half_size;
        let min = position - zone_half_size;
        let max = position + zone_half_size;
        let first_object = zone_objects.len() as i32;
        let final_object = first_object + vec.len() as i32;
        zone_objects.append(&mut vec);
        let zone = SDFZoneDefinitions {
            min,
            max,
            first_object,
            final_object,
        };
        active_zones.push(zone);
    }

    // for x in 0..ZONES_PER_DIMENSION {
    //     for y in 0..ZONES_PER_DIMENSION {
    //         for z in 0..ZONES_PER_DIMENSION {
    //             let offset = Vec3::new(x as f32, y as f32, z as f32);
    //             let position = (offset / (ZONES_PER_DIMENSION as f32)) * settings.max_size
    //                 + bounds_min
    //                 + zone_half_size;
    //             let mut found_in_zone = false;
    //             let mut first_in_zone = 0;
    //             let mut count_in_zone = 0;
    //             let zone_min = position - zone_half_size;
    //             let zone_max = position + zone_half_size;
    //             for (obj, (_, bounds, _)) in objects.iter().enumerate() {
    //                 if bounds.center.distance(position) < bounds.radius * 2. + effective_radius {
    //                     if !found_in_zone {
    //                         first_in_zone = zone_objects.len();
    //                         found_in_zone = true;
    //                     }
    //                     count_in_zone += 1;
    //                     zone_objects.push(obj as i32);
    //                 }
    //             }
    //             if found_in_zone {
    //                 let zone = SDFZoneDefinitions {
    //                     min: zone_min,
    //                     max: zone_max,
    //                     first_object: first_in_zone as i32,
    //                     final_object: (first_in_zone + count_in_zone) as i32,
    //                 };
    //                 active_zones.push(zone);
    //             }
    //         }
    //     }
    // }

    if zone_objects.is_empty() {
        zone_objects.push(0);
    }
    let num_zones = active_zones.len();
    if num_zones == 0 {
        active_zones.push(SDFZoneDefinitions::default())
    }

    let zone_object_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Zone Objects"),
        contents: bytemuck::cast_slice(
            zone_objects
                .iter()
                .map(|a| a.as_std140())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        usage: BufferUsages::STORAGE,
    });

    let zone_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Zones"),
        contents: bytemuck::cast_slice(
            active_zones
                .iter()
                .map(|a| a.as_std140())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        usage: BufferUsages::STORAGE,
    });

    println!("Num Zones: {}", num_zones);

    let num_zones = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Num Zones"),
        contents: bytemuck::cast_slice(&[(NumZones {
            num_zones: num_zones as i32,
            zone_radius,
            zone_size: zone_half_size * 2.,
            zone_origin: bounds_min,
            zones_per_dimension: ZONES_PER_DIMENSION as i32,
        })
        .as_std140()]),
        usage: BufferUsages::UNIFORM,
    });

    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("Zone Bind Group"),
        layout: &sdf_pipeline.zone_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: zone_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: zone_object_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: num_zones.as_entire_binding(),
            },
        ],
    });
    zones.zone_group = Some(bind_group);
}

fn prepare_sdf_origin(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform), With<SDFBakeOrigin>>,
    settings: Res<SDFBakerSettings>,
    mut origin: ResMut<SDFBakedLayerOrigins>,
) {
    let origin_transform = query.get_single();
    let transform = match origin_transform {
        Ok((_, transform)) => transform.translation,
        Err(_) => Vec3::ZERO,
    };
    let dist = (origin.origin - transform).abs();
    let bounds = settings.max_size / 3.;
    if dist.x > bounds.x || dist.y > bounds.y || dist.z > bounds.z {
        origin.origin = (transform * settings.max_size / settings.layer_size).floor()
            * settings.layer_size
            / settings.max_size;
        commands.spawn().insert(ReBakeSDF);
    }
}

fn prepare_bake(query: Query<Entity, With<ReBakeSDF>>, mut rebake: ResMut<ReBakeSDFResource>) {
    let exists = query.get_single().is_ok();
    if exists {
        println!("Setting up new bake");
    }
    rebake.rebake = exists;
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
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            },
        );
        let view = texture.default_view.clone();
        let storage = texture.texture.create_view(&TextureViewDescriptor {
            label: Some("Baked SDF StorageDescriptor"),
            format: Some(TextureFormat::R8Unorm),
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
    _settings: Res<SDFBakerSettings>,
    _textures: ResMut<SDFTextures>,
    _queue: ResMut<RenderQueue>,
    _sdf_roots: Query<(&SDFRootTransform, &SDFObjectTree)>,
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
    if let Some(storage) = &textures.storage {
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
                    resource: BindingResource::TextureView(storage),
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
        _graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        // let rebake = world.get_resource::<ReBakeSDFResource>();
        // if rebake.is_none() || !rebake.unwrap().rebake {
        //     return Ok(());
        // }
        println!("Baking...");
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
        let zone_binding = world
            .get_resource::<SDFZones>()
            .expect("Zones should exist")
            .zone_group
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
        pass.set_bind_group(2, &zone_binding, &[0, 0, 0]);
        pass.dispatch(
            (settings.layer_size.x / 8.).ceil() as u32,
            (settings.layer_size.y / 8.).ceil() as u32,
            (settings.layer_size.z * (settings.num_layers as f32) / 8.).ceil() as u32,
        );
        Ok(())
    }
}
