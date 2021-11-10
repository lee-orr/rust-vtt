use bevy::{asset::Asset, core_pipeline::{SetItemPipeline, Transparent2d, Transparent3d}, ecs::system::lifetimeless::SRes, prelude::{AssetServer, Assets, Entity, FromWorld, Plugin, Query, Res}, render2::{RenderApp, RenderStage, color::Color, render_asset::RenderAssets, render_phase::{AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase}, render_resource::{BlendComponent, BlendFactor, BlendOperation, BlendState, CachedPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Face, FragmentState, FrontFace, MultisampleState, PolygonMode, PrimitiveState, PrimitiveTopology, RawRenderPipelineDescriptor, RenderPipelineCache, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StencilFaceState, StencilState, TextureFormat, VertexState}, renderer::RenderDevice, texture::BevyDefault, view::ExtractedView}};

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
    pipeline: CachedPipelineId,
}

impl FromWorld for SDFPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("sdf_shader.wgsl");

        let descriptor = RenderPipelineDescriptor {
            label: Some("SDF Pipeline".into()),
            layout: None,
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
            depth_stencil:  Some(DepthStencilState {
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
            pipeline: pipeline_cache.queue(descriptor),
        }
    }
}

type DrawSDFCommand = (
    SetItemPipeline,
    DrawSDF
);

pub struct DrawSDF;
impl RenderCommand<Transparent3d> for DrawSDF {
    type Param = ();

    fn render<'w>(
        _view: bevy::prelude::Entity,
        _item: &Transparent3d,
        _: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render2::render_phase::TrackedRenderPass<'w>,
    ) {
        pass.draw(0..3, 0..1);
    }
}

pub fn queue_sdf(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    sdf_pipeline: Res<SDFPipeline>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_sdf = transparent_3d_draw_functions
        .read()
        .get_id::<DrawSDFCommand>()
        .unwrap();
    for (_, mut transparent_phase) in views.iter_mut() {
        transparent_phase.add(Transparent3d {
            distance: 0.,
            pipeline: sdf_pipeline.pipeline,
            entity: Entity::new(0),
            draw_function: draw_sdf,
        });
    }
}
