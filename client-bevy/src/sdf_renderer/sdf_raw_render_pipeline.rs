use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::lifetimeless::{Read, SQuery, SRes},
    math::Vec2,
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::{shape, Mesh},
        render_asset::RenderAssets,
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
    CompareFunction, DepthBiasState, DepthStencilState, Face, FrontFace, MultisampleState,
    PolygonMode, PrimitiveState, PrimitiveTopology, StencilState, TextureFormat, VertexAttribute,
    VertexFormat, VertexStepMode,
};

use super::{
    sdf_brush_binding::{BrushBindingGroup, SDFBrushBindingLayout},
    sdf_lights::{LightBindingGroup, SDFLightBindingLayout},
    sdf_object_zones::{SDFZones, ZoneSettings},
    sdf_view_binding::{SDFViewBinding, SDFViewLayout, ViewExtensionUniformOffset},
};

pub struct SDFRawRenderPipelinePlugin;

impl Plugin for SDFRawRenderPipelinePlugin {
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
            include_str!("shaders/general/raw_sdf_bindings.wgsl"),
            include_str!("shaders/vertex/vertex_full_screen.wgsl"),
            include_str!("shaders/general/sdf_calculator_painted.wgsl"),
            include_str!("shaders/general/sdf_raymarch.wgsl"),
            include_str!("shaders/fragment/fragment_raymarch_calculate_sdf_lights.wgsl"),
        ));
        shaders.set_untracked(SDF_SHADER_HANDLE, shader);

        app.sub_app(RenderApp)
            .init_resource::<SDFPipelineDefinitions>()
            .add_render_command::<Opaque3d, DrawSDFCommand>()
            .add_system_to_stage(RenderStage::Queue, queue_sdf);
    }
}

pub struct SDFPipelineDefinitions {
    pipeline: CachedPipelineId,
}

pub const SDF_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564647005696);
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
        let light_layout = world
            .get_resource::<SDFLightBindingLayout>()
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
            label: Some("SDF Raw Render Pipeline".into()),
            layout: Some(vec![view_layout, brush_layout, zone_layout, light_layout]),
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
                unclipped_depth: false,
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
        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        Self {
            pipeline: pipeline_cache.queue(descriptor),
        }
    }
}

type DrawSDFCommand = (SetItemPipeline, DrawSDF);

pub struct DrawSDF;
impl RenderCommand<Opaque3d> for DrawSDF {
    type Param = (
        SQuery<(Read<ViewUniformOffset>, Read<ViewExtensionUniformOffset>)>,
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<SDFViewBinding>>,
        SQuery<Read<BrushBindingGroup>>,
        SQuery<Read<SDFZones>>,
        SQuery<Read<LightBindingGroup>>,
    );

    fn render<'w>(
        view: bevy::prelude::Entity,
        _item: &Opaque3d,
        (view_offsets, meshes, view_binding, brush_binding, zone_binding, light_binding): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(bindings) = brush_binding.iter().next() {
            pass.set_bind_group(1, &bindings.binding, &[0, 0]);
        }
        if let Some(zones) = zone_binding.iter().next() {
            pass.set_bind_group(2, &zones.zone_group, &[0, 0, 0]);
        }
        if let Some(lights) = light_binding.iter().next() {
            pass.set_bind_group(3, &lights.binding, &[0, 0]);
        }

        if let Ok((view_uniform, view_extension_uniform)) = view_offsets.get(view) {
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
            pipeline: sdf_pipeline.pipeline,
            entity,
            draw_function: draw_sdf,
        });
    }
}
