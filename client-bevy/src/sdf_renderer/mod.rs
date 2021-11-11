use crevice::std140::AsStd140;

use bevy::{
    asset::Asset,
    core_pipeline::{SetItemPipeline, Transparent2d, Transparent3d},
    ecs::system::lifetimeless::{Read, SQuery, SRes},
    math::Mat4,
    prelude::{AssetServer, Assets, Commands, Entity, FromWorld, Plugin, Query, Res, ResMut},
    render2::{
        color::Color,
        render_asset::RenderAssets,
        render_phase::{AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent,
            BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferSize,
            CachedPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, Face, FragmentState, FrontFace, MultisampleState, PolygonMode,
            PrimitiveState, PrimitiveTopology, RawRenderPipelineDescriptor, RenderPipelineCache,
            RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages,
            StencilFaceState, StencilState, TextureFormat, VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{self, ExtractedView, ViewUniformOffset, ViewUniforms},
        RenderApp, RenderStage,
    },
};

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFPipeline>()
            .add_render_command::<Transparent3d, DrawSDFCommand>()
            .add_system_to_stage(RenderStage::Queue, queue_sdf);
    }
}

pub struct SDFPipeline {
    view_layout: BindGroupLayout,
    pipeline: CachedPipelineId,
}

impl FromWorld for SDFPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("sdf_shader.wgsl");
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline Bind Group Layout"),
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
           /* BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                    // Context: https://github.com/LPGhatguy/crevice/issues/29
                    min_binding_size: BufferSize::new(64),
                },
                count: None,
            },*/
            ],
        });

        let descriptor = RenderPipelineDescriptor {
            label: Some("SDF Pipeline".into()),
            layout: Some(vec![view_layout.clone()]),
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: Vec::new(),
                entry_point: "vs_main".into(),
                buffers: Vec::new(),
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
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
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
                shader: shader.clone(),
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
        SDFPipeline {
            view_layout,
            pipeline: pipeline_cache.queue(descriptor),
        }
    }
}

type DrawSDFCommand = (SetItemPipeline, DrawSDF);

pub struct DrawSDF;
impl RenderCommand<Transparent3d> for DrawSDF {
    type Param = SQuery<(Read<ViewUniformOffset>, Read<SDFViewBinding>)>;

    fn render<'w>(
        _view: bevy::prelude::Entity,
        _item: &Transparent3d,
        query: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render2::render_phase::TrackedRenderPass<'w>,
    ) {
        let (view_uniform, view_binding) = query.get(_view).unwrap();
        pass.set_bind_group(0, &view_binding.binding, &[view_uniform.offset]);
        pass.draw(0..3, 0..1);
    }
}

pub struct SDFViewBinding {
    binding: BindGroup,
}

#[derive(Clone, AsStd140)]
pub struct ViewExtension {
    inverted_matrix: Mat4,
}

pub fn queue_sdf(
    mut commands: Commands,
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    sdf_pipeline: Res<SDFPipeline>,
    view_uniforms: Res<ViewUniforms>,
    render_device: Res<RenderDevice>,
    mut views: Query<(Entity, &ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_sdf = transparent_3d_draw_functions
        .read()
        .get_id::<DrawSDFCommand>()
        .unwrap();
    if let Some(binding_resource) = view_uniforms.uniforms.binding() {
        for (entity, view, mut transparent_phase) in views.iter_mut() {
            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("View Bind Group"),
                layout: &sdf_pipeline.view_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding_resource.clone(),
                }],
            });
            let view_binding = SDFViewBinding {
                binding: view_bind_group,
            };
            commands.entity(entity).insert(view_binding);

            transparent_phase.add(Transparent3d {
                distance: 0.,
                pipeline: sdf_pipeline.pipeline,
                entity: entity,
                draw_function: draw_sdf,
            });
        }
    }
}
