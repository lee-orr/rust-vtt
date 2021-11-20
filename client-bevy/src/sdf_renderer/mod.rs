pub mod sdf_block_mesher;
pub mod sdf_operation;



use crevice::std140::AsStd140;

use bevy::{
    core_pipeline::{
        draw_3d_graph::{self, node},
        Opaque3d,
    },
    ecs::system::lifetimeless::{Read, SQuery, SRes},
    math::{Mat4, Vec2},
    prelude::{
        Assets, Commands, CoreStage, Entity, FromWorld, HandleUntyped, Plugin,
        Query, QueryState, Res, ResMut, With, World,
    },
    reflect::TypeUuid,
    render2::{
        camera::PerspectiveProjection,
        mesh::{shape, Mesh},
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraph, SlotInfo, SlotType},
        render_phase::{
            AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase, SetItemPipeline,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent,
            BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType, BufferSize,
            CachedPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, DynamicUniformVec, Face, FragmentState, FrontFace, MultisampleState,
            PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipelineCache,
            RenderPipelineDescriptor, Shader, StencilState, TextureFormat,
            TextureView, VertexBufferLayout, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, CachedTexture, TextureCache},
        view::{ExtractedView, ViewUniformOffset, ViewUniforms},
        RenderApp, RenderStage,
    },
};

use wgpu::{
    util::BufferInitDescriptor, BindingResource, BufferUsages, Color, Extent3d,
    FilterMode, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, SamplerDescriptor, ShaderStages,
    TextureDescriptor, TextureUsages, VertexAttribute, VertexFormat, VertexStepMode,
};

use crate::sdf_renderer::{
    sdf_block_mesher::{extract_gpu_blocks},
    sdf_operation::{
        construct_sdf_object_tree, extract_gpu_node_trees, mark_dirty_object, BrushSettings, SDFRootTransform, Std140GpuSDFNode,
    },
};

use self::{
    sdf_block_mesher::{GpuSDFBlock, Std140GpuSDFBlock},
    sdf_operation::{GpuSDFNode, SDFObjectTree, TRANSFORM_WARP},
};

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let shader = Shader::from_wgsl(format!(
            "{}{}{}{}{}",
            include_str!("structs.wgsl"),
            include_str!("vertex_full_screen.wgsl"),
            include_str!("sdf_calculator.wgsl"),
            include_str!("sdf_raymarch_use_secondary_hits.wgsl"),
            include_str!("full_fragment_secondary_hits.wgsl")
        ));
        shaders.set_untracked(SDF_SHADER_HANDLE, shader);
        let shader = Shader::from_wgsl(format!(
            "{}{}{}{}{}",
            include_str!("structs.wgsl"),
            include_str!("vertex_full_screen.wgsl"),
            include_str!("sdf_calculator.wgsl"),
            include_str!("sdf_raymarch_find_secondary_hits.wgsl"),
            include_str!("depth_fragment_second_hit.wgsl")
        ));
        shaders.set_untracked(SDF_PREPASS_SHADER_HANDLE, shader);
        let mut meshes = app.world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let mesh = Mesh::from(shape::Quad {
            size: 2. * Vec2::ONE,
            flip: false,
        });
        println!("Mesh: {:?}", mesh);
        meshes.set_untracked(SDF_CUBE_MESH_HANDLE, mesh);
        app
            // .add_plugin(SdfBlockMeshingPlugin)
            .add_system_to_stage(CoreStage::PostUpdate, mark_dirty_object)
            .add_system_to_stage(CoreStage::Last, construct_sdf_object_tree);
        let render_app = app
            .sub_app(RenderApp)
            .init_resource::<SDFPipeline>()
            .init_resource::<ViewExtensionUniforms>()
            .init_resource::<BrushUniforms>()
            .init_resource::<BrushBindingGroupResource>()
            .add_render_command::<Opaque3d, DrawSDFCommand>()
            .add_system_to_stage(RenderStage::Extract, extract_gpu_blocks)
            .add_system_to_stage(RenderStage::Extract, extract_gpu_node_trees)
            .add_system_to_stage(RenderStage::Prepare, prepare_brush_uniforms)
            .add_system_to_stage(RenderStage::Prepare, prepare_view_extensions)
            .add_system_to_stage(RenderStage::Prepare, prepare_depth_pass_texture)
            .add_system_to_stage(RenderStage::Queue, queue_sdf)
            .add_system_to_stage(RenderStage::Queue, queue_brush_bindings);

        let depth_pre_pass_node = DepthPrePassNode::new(&mut render_app.world);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        let draw_3d_graph = graph.get_sub_graph_mut(draw_3d_graph::NAME);
        if let Some(draw_3d_graph) = draw_3d_graph {
            draw_3d_graph.add_node(DepthPrePassNode::NAME, depth_pre_pass_node);
            let input_node_id = draw_3d_graph.input_node().unwrap().id;
            draw_3d_graph
                .add_slot_edge(
                    input_node_id,
                    draw_3d_graph::input::VIEW_ENTITY,
                    DepthPrePassNode::NAME,
                    DepthPrePassNode::IN_VIEW,
                )
                .unwrap();
            draw_3d_graph
                .add_node_edge(DepthPrePassNode::NAME, node::MAIN_PASS)
                .unwrap();
        }
    }
}

pub struct SDFPipeline {
    view_layout: BindGroupLayout,
    brush_layout: BindGroupLayout,
    depth_layout: BindGroupLayout,
    pipeline: CachedPipelineId,
    prepass: CachedPipelineId,
}
pub const SDF_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564647005696);
pub const SDF_PREPASS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1356745757609005696);
pub const SDF_CUBE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 1674555646470534696);

impl FromWorld for SDFPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();
        let shader = SDF_SHADER_HANDLE.typed::<Shader>();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline View Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
            ],
        });

        let depth_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                    ty: BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let brush_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline BrushBind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
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
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let (vertex_array_stride, vertex_attributes) = (
            32,
            vec![
                // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 0,
                },
                // Normal
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                },
                // Uv
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 2,
                },
            ],
        );

        let descriptor = RenderPipelineDescriptor {
            label: Some("SDF Pipeline".into()),
            layout: Some(vec![
                view_layout.clone(),
                brush_layout.clone(),
                depth_layout.clone(),
            ]),
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: Vec::new(),
                entry_point: "vs_main".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: vertex_array_stride,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vertex_attributes.clone(),
                }],
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: Vec::new(),
                entry_point: "fs_main".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
        };
        let prepass_shader = SDF_PREPASS_SHADER_HANDLE.typed::<Shader>();
        let prepass_descriptor = RenderPipelineDescriptor {
            label: Some("SDF Prepass Pipeline".into()),
            layout: Some(vec![view_layout.clone(), brush_layout.clone()]),
            vertex: VertexState {
                shader: prepass_shader.clone(),
                shader_defs: Vec::new(),
                entry_point: "vs_main".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: vertex_array_stride,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vertex_attributes,
                }],
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: prepass_shader,
                shader_defs: Vec::new(),
                entry_point: "fs_main".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
            }),
        };
        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        SDFPipeline {
            view_layout,
            brush_layout,
            depth_layout,
            pipeline: pipeline_cache.queue(descriptor),
            prepass: pipeline_cache.queue(prepass_descriptor),
        }
    }
}

type DrawSDFCommand = (SetItemPipeline, DrawSDF);

pub struct DrawSDF;
impl RenderCommand<Opaque3d> for DrawSDF {
    type Param = (
        SQuery<(
            Read<ViewUniformOffset>,
            Read<ViewExtensionUniformOffset>,
            Read<SDFViewBinding>,
            Read<ViewDepthPass1>,
        )>,
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<SDFBrushBinding>>,
    );

    fn render<'w>(
        view: bevy::prelude::Entity,
        _item: &Opaque3d,
        (query, meshes, bindings): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render2::render_phase::TrackedRenderPass<'w>,
    ) {
        if let Some(bindings) = bindings.iter().next() {
            pass.set_bind_group(1, &bindings.binding, &[0, 0, 0]);
        }
        if let Ok((view_uniform, view_extension_uniform, view_binding, depth_pass)) =
            query.get(view)
        {
            pass.set_bind_group(
                0,
                &view_binding.binding,
                &[view_uniform.offset, view_extension_uniform.offset],
            );
            pass.set_bind_group(2, &depth_pass.bind_group, &[]);
            let mesh = meshes
                .into_inner()
                .get(&SDF_CUBE_MESH_HANDLE.typed::<Mesh>())
                .unwrap();
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            if let Some(index_info) = &mesh.index_info {
                pass.set_index_buffer(index_info.buffer.slice(..), 0, index_info.index_format);
                pass.draw_indexed(0..index_info.count, 0, 0..1);
            }
        }
    }
}

pub struct SDFViewBinding {
    binding: BindGroup,
    pub num_blocks: u32,
}

#[derive(Clone, AsStd140)]
pub struct ViewExtension {
    view_proj_inverted: Mat4,
    proj_inverted: Mat4,
    cone_scaler: f32,
    pixel_size: f32,
}

#[derive(Default)]
pub struct ViewExtensionUniforms {
    pub uniforms: DynamicUniformVec<ViewExtension>,
}

pub struct ViewExtensionUniformOffset {
    pub offset: u32,
}

#[derive(Default)]
pub struct BrushUniforms {
    pub brushes: Option<Buffer>,
    pub settings: DynamicUniformVec<BrushSettings>,
    pub blocks: Option<Buffer>,
}

pub struct SDFBrushBinding {
    binding: BindGroup,
}

#[derive(Default)]
pub struct BrushBindingGroupResource {
    binding: Option<BindGroup>,
}

fn prepare_view_extensions(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_extension_uniforms: ResMut<ViewExtensionUniforms>,
    views: Query<(Entity, &ExtractedView, Option<&PerspectiveProjection>)>,
) {
    view_extension_uniforms.uniforms.clear();
    for (entity, camera, perspective) in views.iter() {
        let projection = camera.projection;
        let view_proj = projection * camera.transform.compute_matrix().inverse();
        let max_pixels = if camera.width > camera.height {
            camera.width
        } else {
            camera.height
        };
        let view_extension_uniform_offset = ViewExtensionUniformOffset {
            offset: view_extension_uniforms.uniforms.push(ViewExtension {
                view_proj_inverted: view_proj.inverse(),
                proj_inverted: projection.inverse(),
                cone_scaler: if let Some(p) = perspective {
                    p.fov.tan()
                } else {
                    1.
                },
                pixel_size: 1.0 / (max_pixels as f32),
            }),
        };
        commands
            .entity(entity)
            .insert(view_extension_uniform_offset);
    }
    view_extension_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

fn prepare_brush_uniforms(
    mut brush_uniforms: ResMut<BrushUniforms>,
    objects: Query<(&SDFObjectTree, &SDFRootTransform)>,
    block_query: Query<&GpuSDFBlock>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView)>,
) {
    let objects: Vec<(&SDFObjectTree, &SDFRootTransform)> = objects.iter().collect();
    let object_count = objects.len();
    let mut index_so_far = object_count;
    let mut brush_vec: Vec<GpuSDFNode> = Vec::new();
    for (tree, transform) in &objects {
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
    }
    for (tree, _) in &objects {
        for node in &tree.tree {
            brush_vec.push(node.clone());
        }
    }
    let mut blocks: Vec<Std140GpuSDFBlock> = Vec::<Std140GpuSDFBlock>::new();
    if let Some((_, view)) = views.iter().next() {
        let position = view.transform.translation;
        let mut tmp_blocks: Vec<(&GpuSDFBlock, f32)> = block_query
            .iter()
            .map(|block| (block, (block.position - position).length()))
            .collect();
        tmp_blocks.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
        blocks = tmp_blocks.iter().map(|(val, _)| val.as_std140()).collect();
    } else {
        blocks = block_query.iter().map(|block| block.as_std140()).collect();
    }

    let mut brushes: Vec<Std140GpuSDFNode> = brush_vec.iter().map(|val| val.as_std140()).collect();

    if brushes.is_empty() {
        brushes.push(GpuSDFNode::default().as_std140());
    }
    if blocks.is_empty() {
        blocks.push(GpuSDFBlock::default().as_std140());
    }
    brush_uniforms.settings.clear();
    brush_uniforms.settings.push(BrushSettings {
        num_brushes: object_count as i32,
    });
    brush_uniforms
        .settings
        .write_buffer(&render_device, &render_queue);

    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Brush Buffer"),
        contents: bytemuck::cast_slice(brushes.as_slice()),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let block_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Block Buffer"),
        contents: bytemuck::cast_slice(blocks.as_slice()),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    brush_uniforms.brushes = Some(buffer);
    brush_uniforms.blocks = Some(block_buffer);
}

pub fn queue_brush_bindings(
    mut commands: Commands,
    buffers: Res<BrushUniforms>,
    render_device: Res<RenderDevice>,
    sdf_pipeline: Res<SDFPipeline>,
    mut brush_binding: ResMut<BrushBindingGroupResource>,
) {
    if let (Some(brushes), settings, Some(blocks)) =
        (&buffers.brushes, &buffers.settings, &buffers.blocks)
    {
        let brush_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Brush Bind Group"),
            layout: &sdf_pipeline.brush_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: brushes.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: settings.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: blocks.as_entire_binding(),
                },
            ],
        });
        brush_binding.binding = Some(brush_bind_group.clone());
        commands.spawn().insert(SDFBrushBinding {
            binding: brush_bind_group,
        });
    }
}

pub struct ViewDepthPass1 {
    pub texture: CachedTexture,
    pub second_hit_texture: CachedTexture,
    pub view: TextureView,
    pub second_hit_view: TextureView,
    pub bind_group: BindGroup,
}

const DEPTH_PASS_RATIO: u32 = 8;

pub fn prepare_depth_pass_texture(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedView)>,
    mut texture_cache: ResMut<TextureCache>,
    sdf_pipeline: Res<SDFPipeline>,
) {
    for (entity, view) in views.iter() {
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Depth Pass 1"),
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
        let second_hit_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Second Hit"),
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: view.width / DEPTH_PASS_RATIO as u32,
                    height: view.height / DEPTH_PASS_RATIO as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );
        let second_hit_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("Second Hit Sampler"),
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let view = texture.default_view.clone();
        let second_hit_view = second_hit_texture.default_view.clone();
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("Depth Pass 1 Binding Group"),
            layout: &sdf_pipeline.depth_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&second_hit_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&second_hit_sampler),
                },
            ],
        });
        commands.entity(entity).insert(ViewDepthPass1 {
            texture,
            second_hit_texture,
            view,
            second_hit_view,
            bind_group,
        });
    }
}

pub fn queue_sdf(
    mut commands: Commands,
    transparent_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    sdf_pipeline: Res<SDFPipeline>,
    view_uniforms: Res<ViewUniforms>,
    view_extension_uniforms: Res<ViewExtensionUniforms>,
    render_device: Res<RenderDevice>,
    blocks: Query<&GpuSDFBlock>,
    mut views: Query<(Entity, &ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_sdf = transparent_3d_draw_functions
        .read()
        .get_id::<DrawSDFCommand>()
        .unwrap();
    if let (Some(binding_resource), Some(extension_binding_resource)) = (
        view_uniforms.uniforms.binding(),
        view_extension_uniforms.uniforms.binding(),
    ) {
        for (entity, _view, mut opaque_phase) in views.iter_mut() {
            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("View Bind Group"),
                layout: &sdf_pipeline.view_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: binding_resource.clone(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: extension_binding_resource.clone(),
                    },
                ],
            });
            let view_binding = SDFViewBinding {
                binding: view_bind_group,
                num_blocks: blocks.iter().count() as u32,
            };
            commands.entity(entity).insert(view_binding);

            opaque_phase.add(Opaque3d {
                distance: 0.,
                pipeline: sdf_pipeline.pipeline,
                entity,
                draw_function: draw_sdf,
            });
        }
    }
}

pub struct DepthPrePassNode {
    pub view_query: QueryState<
        (
            &'static SDFViewBinding,
            &'static ViewDepthPass1,
            &'static ViewUniformOffset,
            &'static ViewExtensionUniformOffset,
        ),
        With<ExtractedView>,
    >,
}

impl DepthPrePassNode {
    pub const IN_VIEW: &'static str = "view";
    pub const NAME: &'static str = "DEPTH_PRE_PASS_NODE";

    pub fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for DepthPrePassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(DepthPrePassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        let view_entity = graph
            .get_input_entity(Self::IN_VIEW)
            .expect("Should find attached entity");
        let pipeline = world
            .get_resource::<SDFPipeline>()
            .expect("Pipeline Should Exist");
        let brush_binding = world
            .get_resource::<BrushBindingGroupResource>()
            .expect("Binding Should Exist");
        let brush_binding = brush_binding.binding.clone().unwrap();
        let pipeline_cache = world
            .get_resource::<RenderPipelineCache>()
            .expect("Pipeline Cache Should Exist");
        let meshes = world
            .get_resource::<RenderAssets<Mesh>>()
            .expect("Mesh Assets");
        let (view_binding, depth_pass, view_offset, extension_offset) = self
            .view_query
            .get_manual(world, view_entity)
            .expect("View Entity Should Exist");

        {
            let pass_descriptor = RenderPassDescriptor {
                label: Some("depth_prepass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &depth_pass.second_hit_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth_pass.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };
            let mut pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            pass.set_bind_group(
                0,
                &view_binding.binding,
                &[view_offset.offset, extension_offset.offset],
            );
            pass.set_bind_group(1, &brush_binding, &[0, 0, 0]);
            pass.set_pipeline(pipeline_cache.get(pipeline.prepass).unwrap());
            let mesh = meshes.get(&SDF_CUBE_MESH_HANDLE.typed::<Mesh>()).unwrap();
            pass.set_vertex_buffer(0, *mesh.vertex_buffer.slice(..));
            if let Some(index_info) = &mesh.index_info {
                pass.set_index_buffer(*index_info.buffer.slice(..), index_info.index_format);
                pass.draw_indexed(0..index_info.count, 0, 0..1);
            }
        }

        Ok(())
    }
}
