use std::collections::VecDeque;

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
        render_graph::{Node, NodeId, RenderGraph, SlotInfo},
        render_phase::{
            AddRenderCommand, DrawFunctions, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline,
        },
        render_resource::{
            CachedPipelineId, ComputePipeline, FragmentState, RenderPipelineCache,
            RenderPipelineDescriptor, Shader, VertexBufferLayout, VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ExtractedView, ViewUniformOffset},
        RenderApp, RenderStage,
    },
};
use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
    CompareFunction, ComputePassDescriptor, ComputePipelineDescriptor, DepthBiasState,
    DepthStencilState, Face, FrontFace, MultisampleState, PolygonMode,
    PrimitiveState, PrimitiveTopology, ShaderModuleDescriptor, ShaderSource, StencilState, TextureFormat,
    VertexAttribute, VertexFormat, VertexStepMode,
};

use crate::sdf_renderer::sdf_lights::SDFLightBindingLayout;

use super::{
    sdf_brush_binding::{BrushBindingGroup, SDFBrushBindingLayout},
    sdf_depth_binding::{DepthBindingGroup, SDFDepthBindingLayout},
    sdf_object_zones::{SDFZones},
    sdf_octtree_binding::{OcttreeBindingGroups, SDFOcttreeBindingLayout, TREE_DEPTH},
    sdf_view_binding::{SDFViewBinding, SDFViewLayout, ViewExtensionUniformOffset}, sdf_lights::LightBindingGroup,
};

pub struct SDFRawOcttreePipelinePlugin;

impl Plugin for SDFRawOcttreePipelinePlugin {
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
            include_str!("shaders/general/raw_octree_render_bindings.wgsl"),
            include_str!("shaders/vertex/vertex_full_screen.wgsl"),
            include_str!("shaders/general/sdf_calculator_object_list.wgsl"),
            include_str!("shaders/general/sdf_raymarch.wgsl"),
            include_str!("shaders/fragment/fragment_raymarch_calculate_sdf_read_depth.wgsl"),
        ));
        shaders.set_untracked(SDF_RENDER_SHADER_HANDLE, shader);
        let shader = Shader::from_wgsl(format!(
            "{}{}{}{}",
            include_str!("shaders/general/structs.wgsl"),
            include_str!("shaders/general/raw_octree_render_bindings.wgsl"),
            include_str!("shaders/general/sdf_calculator_octree_painted.wgsl"),
            include_str!("shaders/compute/raw_octree_baker.wgsl"),
        ));
        shaders.set_untracked(SDF_BAKE_SHADER_HANDLE, shader);

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<SDFPipelineDefinitions>()
            .add_render_command::<Opaque3d, DrawSDFCommand>()
            .add_system_to_stage(RenderStage::Queue, queue_sdf);

        let mut nodes: VecDeque<OcttreePassNode> = VecDeque::new();

        for layer in 0..TREE_DEPTH {
            let node = OcttreePassNode::new(&mut render_app.world, layer);
            nodes.push_back(node);
        }

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        let draw_3d_graph = graph.get_sub_graph_mut(draw_3d_graph::NAME);
        if let Some(draw_3d_graph) = draw_3d_graph {
            let mut name: Option<NodeId> = None;
            loop {
                let node = nodes.pop_front();
                if let Some(node) = node {
                    let new_name = node.name.clone();
                    let new_name = std::borrow::Cow::Owned(new_name);
                    let id = draw_3d_graph.add_node(new_name, node);
                    if let Some(name) = name {
                        draw_3d_graph.add_node_edge(name, id).unwrap();
                    }
                    name = Some(id);
                } else {
                    break;
                }
            }
            if let Some(name) = name {
                draw_3d_graph.add_node_edge(name, node::MAIN_PASS).unwrap();
            }
        }
    }
}

pub struct SDFPipelineDefinitions {
    main_pipeline: CachedPipelineId,
    bake_pipeline: ComputePipeline,
}

pub const SDF_RENDER_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564647005696);
pub const SDF_BAKE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564645335691);
pub const SDF_CUBE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 1674555646470534696);

impl FromWorld for SDFPipelineDefinitions {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();

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
        let tree_layouts = world.get_resource::<SDFOcttreeBindingLayout>().unwrap();
        let light_layout = world
            .get_resource::<SDFLightBindingLayout>()
            .unwrap()
            .layout
            .clone();
        let msaa = if let Some(msaa) = world.get_resource::<Msaa>() {
            msaa.clone()
        } else {
            Msaa::default()
        };
        println!("MSAA: {}", msaa.samples);

        let shader = SDF_RENDER_SHADER_HANDLE.typed::<Shader>();

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
            label: Some("SDF Octtree Render Pipeline".into()),
            layout: Some(vec![
                view_layout,
                tree_layouts.read_layout.clone(),
                light_layout,
            ]),
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
                count: msaa.samples,
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

        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = format!(
            "{}{}{}",
            include_str!("shaders/general/structs.wgsl"),
            include_str!("shaders/general/raw_octree_render_bindings.wgsl"),
            //include_str!("shaders/general/sdf_calculator_octree_painted.wgsl"),
            include_str!("shaders/compute/raw_octree_baker.wgsl"),
        );
        let shader = ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader.as_str()));
        let module = render_device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Octree Baker Module"),
            source: shader,
        });
        let compute_pipeline_layout =
            render_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[&brush_layout, &tree_layouts.write_layout.clone(), &tree_layouts.dispatch_layout.clone()],
                push_constant_ranges: &[],
            });
        let bake_pipeline_descriptor = ComputePipelineDescriptor {
            label: Some("SDF Octree Write Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &module,
            entry_point: "cmp_main",
        };

        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        Self {
            main_pipeline: pipeline_cache.queue(descriptor),
            bake_pipeline: render_device.create_compute_pipeline(&bake_pipeline_descriptor),
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
        )>,
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<SDFViewBinding>>,
        SQuery<Read<OcttreeBindingGroups>>,
        SQuery<Read<LightBindingGroup>>,
    );

    fn render<'w>(
        view: bevy::prelude::Entity,
        _item: &Opaque3d,
        (view_offsets, meshes, view_binding, octtree_binding, light_binding): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(bindings) = octtree_binding.iter().next() {
            pass.set_bind_group(1, &bindings.read_binding, &[0, 0, 0]);
        }
        if let Some(lights) = light_binding.iter().next() {
            pass.set_bind_group(2, &lights.binding, &[0, 0]);
        }

        if let Ok((view_uniform, view_extension_uniform)) =
            view_offsets.get(view)
        {
            if let Some(view_binding) = view_binding.iter().next() {
                pass.set_bind_group(
                    0,
                    &view_binding.binding,
                    &[view_uniform.offset, view_extension_uniform.offset],
                );
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

pub struct OcttreePassNode {
    pub name: String,
    pub depth_layer: u32,
    pub brush_binding: QueryState<&'static BrushBindingGroup>,
    pub tree_binding: QueryState<&'static OcttreeBindingGroups>,
}

impl OcttreePassNode {
    pub const IN_VIEW: &'static str = "view";
    pub const NAME: &'static str = "DEPTH_PRE_PASS_NODE";

    pub fn new(world: &mut World, layer: u32) -> Self {
        Self {
            name: format!("{}_{}", Self::NAME, &layer),
            depth_layer: layer,
            brush_binding: QueryState::new(world),
            tree_binding: QueryState::new(world),
        }
    }
}

impl Node for OcttreePassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![]
    }

    fn update(&mut self, world: &mut World) {
        self.brush_binding.update_archetypes(world);
        self.tree_binding.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let pipeline = world
            .get_resource::<SDFPipelineDefinitions>()
            .expect("Pipeline Should Exist");
        let brush_binding = &self
            .brush_binding
            .iter_manual(world)
            .next()
            .expect("Brushes should be bound")
            .binding;
        let bake_binding = self
            .tree_binding
            .iter_manual(world)
            .next()
            .expect("bindings should exist");
        let depth_layer = self.depth_layer;

        {
            let pass_descriptor = ComputePassDescriptor {
                label: Some(self.name.as_str()),
            };

            let mut pass = render_context
                .command_encoder
                .begin_compute_pass(&pass_descriptor);
            pass.set_pipeline(&pipeline.bake_pipeline);
            pass.set_bind_group(0, brush_binding, &[0, 0]);
            pass.set_bind_group(1, &bake_binding.write_binding, &[0, 0, depth_layer, 0]);
            if self.depth_layer % 2 == 0 {
                pass.set_bind_group(2, &bake_binding.dispatch_2_binding, &[0]);
                pass.dispatch_indirect(&bake_binding.dispatch1, depth_layer.into())
            } else {
                pass.set_bind_group(2, &bake_binding.dispatch_1_binding, &[0]);
                pass.dispatch_indirect(&bake_binding.dispatch2, depth_layer.into())
            }
        }

        Ok(())
    }
}
