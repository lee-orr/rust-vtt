use bevy::{
    core_pipeline::{
        draw_3d_graph::{self, node},
        Opaque3d,
    },
    ecs::system::lifetimeless::{Read, SQuery, SRes},
    math::Vec2,
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::{shape, Mesh},
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraph, SlotInfo, SlotType},
        render_phase::{
            AddRenderCommand, DrawFunctions, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline,
        },
        render_resource::{
            CachedPipelineId, FragmentState, RenderPipelineCache, RenderPipelineDescriptor, Shader,
            VertexBufferLayout, VertexState,
        },
        texture::BevyDefault,
        view::{ExtractedView, ViewUniformOffset},
        RenderApp, RenderStage,
    },
};
use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
    CompareFunction, DepthBiasState, DepthStencilState, Face, FrontFace, LoadOp, MultisampleState,
    Operations, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StencilState, TextureFormat,
    VertexAttribute, VertexFormat, VertexStepMode,
};

use super::{
    sdf_brush_binding::{BrushBindingGroup, SDFBrushBindingLayout},
    sdf_depth_binding::{DepthBindingGroup, SDFDepthBindingLayout},
    sdf_object_zones::{SDFZones, ZoneSettings},
    sdf_view_binding::{SDFViewBinding, SDFViewLayout, ViewExtensionUniformOffset},
};

pub struct SDFRawWithDepthPassPipelinePlugin;

impl Plugin for SDFRawWithDepthPassPipelinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut meshes = app.world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let mesh = Mesh::from(shape::Quad {
            size: 2. * Vec2::ONE,
            flip: false,
        });
        meshes.set_untracked(SDF_CUBE_MESH_HANDLE, mesh);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let shader = Shader::from_wgsl(format!(
            "{}{}{}{}{}{}",
            include_str!("shaders/general/structs.wgsl"),
            include_str!("shaders/general/raw_sdf_depth_bindings.wgsl"),
            include_str!("shaders/vertex/vertex_full_screen.wgsl"),
            include_str!("shaders/general/sdf_calculator_object_list.wgsl"),
            include_str!("shaders/general/sdf_raymarch.wgsl"),
            include_str!("shaders/fragment/fragment_raymarch_calculate_sdf_read_depth.wgsl"),
        ));
        shaders.set_untracked(SDF_SHADER_HANDLE, shader);
        let shader = Shader::from_wgsl(format!(
            "{}{}{}{}{}{}",
            include_str!("shaders/general/structs.wgsl"),
            include_str!("shaders/general/raw_sdf_bindings.wgsl"),
            include_str!("shaders/vertex/vertex_full_screen.wgsl"),
            include_str!("shaders/general/sdf_calculator_object_list.wgsl"),
            include_str!("shaders/general/sdf_raymarch.wgsl"),
            include_str!("shaders/fragment/fragment_raymarch_calculate_sdf_write_depth.wgsl"),
        ));
        shaders.set_untracked(SDF_DEPTH_SHADER_HANDLE, shader);

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<SDFPipelineDefinitions>()
            .add_render_command::<Opaque3d, DrawSDFCommand>()
            .add_system_to_stage(RenderStage::Queue, queue_sdf);

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

pub struct SDFPipelineDefinitions {
    main_pipeline: CachedPipelineId,
    depth_pipeline: CachedPipelineId,
}

pub const SDF_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564647005696);
pub const SDF_DEPTH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564645335691);
pub const SDF_CUBE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 1674555646470534696);

impl FromWorld for SDFPipelineDefinitions {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();

        let zone_layout = { world.get_resource::<ZoneSettings>().unwrap().layout.clone() };

        let view_layout = world
            .get_resource::<SDFViewLayout>()
            .unwrap()
            .layout
            .clone();
        let brush_layout = world
            .get_resource::<SDFBrushBindingLayout>()
            .unwrap()
            .layout
            .clone();
        let depth_layout = world
            .get_resource::<SDFDepthBindingLayout>()
            .unwrap()
            .layout
            .clone();

        let shader = SDF_SHADER_HANDLE.typed::<Shader>();

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
            label: Some("SDF Depth Render Pipeline".into()),
            layout: Some(vec![
                view_layout.clone(),
                brush_layout.clone(),
                zone_layout.clone(),
                depth_layout,
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
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                unclipped_depth: false,
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
        let shader = SDF_DEPTH_SHADER_HANDLE.typed::<Shader>();
        let depth_pass_descriptor = RenderPipelineDescriptor {
            label: Some("SDF Depth Pass Render Pipeline".into()),
            layout: Some(vec![view_layout, brush_layout, zone_layout]),
            vertex: VertexState {
                shader: shader.clone(),
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
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                unclipped_depth: false,
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
                shader,
                shader_defs: Vec::new(),
                entry_point: "fs_main".into(),
                targets: vec![],
            }),
        };
        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        Self {
            main_pipeline: pipeline_cache.queue(descriptor),
            depth_pipeline: pipeline_cache.queue(depth_pass_descriptor),
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
            Read<DepthBindingGroup>,
        )>,
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<SDFViewBinding>>,
        SQuery<Read<BrushBindingGroup>>,
        SQuery<Read<SDFZones>>,
    );

    fn render<'w>(
        view: bevy::prelude::Entity,
        _item: &Opaque3d,
        (view_offsets, meshes, view_binding, brush_binding, zone_binding): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(bindings) = brush_binding.iter().next() {
            pass.set_bind_group(1, &bindings.binding, &[0, 0]);
        }
        if let Some(zones) = zone_binding.iter().next() {
            pass.set_bind_group(2, &zones.zone_group, &[0, 0, 0]);
        }

        if let Ok((view_uniform, view_extension_uniform, depth_binding_group)) =
            view_offsets.get(view)
        {
            if let Some(view_binding) = view_binding.iter().next() {
                pass.set_bind_group(
                    0,
                    &view_binding.binding,
                    &[view_uniform.offset, view_extension_uniform.offset],
                );
                pass.set_bind_group(3, &depth_binding_group.binding, &[]);
                let mesh = meshes
                    .into_inner()
                    .get(&SDF_CUBE_MESH_HANDLE.typed::<Mesh>())
                    .unwrap();
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                if let Some(index_info) = &mesh.index_info {
                    pass.set_index_buffer(index_info.buffer.slice(..), 0, index_info.index_format);
                    pass.draw_indexed(0..index_info.count, 0, 0..1);
                    return RenderCommandResult::Success;
                }
            }
        }
        RenderCommandResult::Failure
    }
}

pub fn queue_sdf(
    transparent_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    sdf_pipeline: Res<SDFPipelineDefinitions>,
    mut views: Query<(Entity, &ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_sdf = transparent_3d_draw_functions
        .read()
        .get_id::<DrawSDFCommand>()
        .unwrap();
    for (entity, _view, mut opaque_phase) in views.iter_mut() {
        opaque_phase.add(Opaque3d {
            distance: 0.,
            pipeline: sdf_pipeline.main_pipeline,
            entity,
            draw_function: draw_sdf,
        });
    }
}

pub struct DepthPrePassNode {
    pub view_query: QueryState<
        (
            &'static DepthBindingGroup,
            &'static ViewUniformOffset,
            &'static ViewExtensionUniformOffset,
        ),
        With<ExtractedView>,
    >,
    pub view_binding: QueryState<&'static SDFViewBinding>,
    pub brush_binding: QueryState<&'static BrushBindingGroup>,
    pub zone_binding: QueryState<&'static SDFZones>,
}

impl DepthPrePassNode {
    pub const IN_VIEW: &'static str = "view";
    pub const NAME: &'static str = "DEPTH_PRE_PASS_NODE";

    pub fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
            view_binding: QueryState::new(world),
            brush_binding: QueryState::new(world),
            zone_binding: QueryState::new(world),
        }
    }
}

impl Node for DepthPrePassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(DepthPrePassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
        self.view_binding.update_archetypes(world);
        self.brush_binding.update_archetypes(world);
        self.zone_binding.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let view_entity = graph
            .get_input_entity(Self::IN_VIEW)
            .expect("Should find attached entity");
        let pipeline = world
            .get_resource::<SDFPipelineDefinitions>()
            .expect("Pipeline Should Exist");
        let brush_binding = &self
            .brush_binding
            .iter_manual(world)
            .next()
            .expect("Brushes should be bound")
            .binding;
        let zone_binding = &self
            .zone_binding
            .iter_manual(world)
            .next()
            .expect("Brushes should be bound")
            .zone_group;
        let view_binding = &self
            .view_binding
            .iter_manual(world)
            .next()
            .expect("Brushes should be bound")
            .binding;
        let pipeline_cache = world
            .get_resource::<RenderPipelineCache>()
            .expect("Pipeline Cache Should Exist");
        let meshes = world
            .get_resource::<RenderAssets<Mesh>>()
            .expect("Mesh Assets");
        let (depth_pass, view_offset, extension_offset) = self
            .view_query
            .get_manual(world, view_entity)
            .expect("View Entity Should Exist");

        {
            let pass_descriptor = RenderPassDescriptor {
                label: Some("depth_prepass"),
                color_attachments: &[],
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
                view_binding,
                &[view_offset.offset, extension_offset.offset],
            );
            pass.set_bind_group(2, zone_binding, &[0, 0, 0]);
            pass.set_bind_group(1, brush_binding, &[0, 0]);
            pass.set_pipeline(pipeline_cache.get(pipeline.depth_pipeline).unwrap());
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
