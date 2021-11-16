pub mod sdf_operation;

use crevice::std140::AsStd140;

use bevy::{
    core_pipeline::{SetItemPipeline, Transparent3d},
    ecs::system::lifetimeless::{Read, SQuery},
    math::Mat4,
    prelude::{Assets, Commands, Entity, FromWorld, HandleUntyped, Plugin, Query, Res, ResMut},
    reflect::TypeUuid,
    render2::{
        camera::PerspectiveProjection,
        render_phase::{AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent,
            BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType, BufferSize,
            CachedPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, DynamicUniformVec, Face, FragmentState, FrontFace, MultisampleState,
            PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipelineCache,
            RenderPipelineDescriptor, Shader, StencilFaceState, StencilState, TextureFormat,
            VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewUniformOffset, ViewUniforms},
        RenderApp, RenderStage,
    },
};

use wgpu::{util::BufferInitDescriptor, BufferUsages, ShaderStages};

use crate::sdf_renderer::sdf_operation::{
    extract_sdf_brushes, BrushSettings, ExtractedSDFBrush, Std140ExtractedSDFBrush,
};

use self::sdf_operation::ExtractedSDFOrder;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let shader = Shader::from_wgsl(include_str!("sdf_shader.wgsl"));
        shaders.set_untracked(SDF_SHADER_HANDLE, shader);
        app.sub_app(RenderApp)
            .init_resource::<SDFPipeline>()
            .init_resource::<ViewExtensionUniforms>()
            .init_resource::<BrushUniforms>()
            .add_render_command::<Transparent3d, DrawSDFCommand>()
            .add_system_to_stage(RenderStage::Extract, extract_sdf_brushes)
            .add_system_to_stage(RenderStage::Prepare, prepare_brush_uniforms)
            .add_system_to_stage(RenderStage::Prepare, prepare_view_extensions)
            .add_system_to_stage(RenderStage::Queue, queue_sdf)
            .add_system_to_stage(RenderStage::Queue, queue_brush_bindings);
    }
}

pub struct SDFPipeline {
    view_layout: BindGroupLayout,
    brush_layout: BindGroupLayout,
    pipeline: CachedPipelineId,
}
pub const SDF_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564647005696);

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
            ],
        });

        let descriptor = RenderPipelineDescriptor {
            label: Some("SDF Pipeline".into()),
            layout: Some(vec![view_layout.clone(), brush_layout.clone()]),
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
        SDFPipeline {
            view_layout,
            brush_layout,
            pipeline: pipeline_cache.queue(descriptor),
        }
    }
}

type DrawSDFCommand = (SetItemPipeline, PrepareSDFBuffer, DrawSDF);

pub struct DrawSDF;
impl RenderCommand<Transparent3d> for DrawSDF {
    type Param = SQuery<(
        Read<ViewUniformOffset>,
        Read<ViewExtensionUniformOffset>,
        Read<SDFViewBinding>,
    )>;

    fn render<'w>(
        _view: bevy::prelude::Entity,
        _item: &Transparent3d,
        query: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render2::render_phase::TrackedRenderPass<'w>,
    ) {
        let (view_uniform, view_extension_uniform, view_binding) = query.get(_view).unwrap();
        pass.set_bind_group(
            0,
            &view_binding.binding,
            &[view_uniform.offset, view_extension_uniform.offset],
        );
        pass.draw(0..3, 0..1);
    }
}

pub struct PrepareSDFBuffer;
impl RenderCommand<Transparent3d> for PrepareSDFBuffer {
    type Param = SQuery<Read<SDFBrushBinding>>;

    fn render<'w>(
        _view: Entity,
        _item: &Transparent3d,
        param: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render2::render_phase::TrackedRenderPass<'w>,
    ) {
        if let Some(bindings) =  param.iter().next() {
            pass.set_bind_group(1, &bindings.binding, &[0, 0]);
        }
    }
}

pub struct SDFViewBinding {
    binding: BindGroup,
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
}

pub struct SDFBrushBinding {
    binding: BindGroup,
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
    brushes: Query<(&ExtractedSDFBrush, &ExtractedSDFOrder)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let mut brushes: Vec<(&ExtractedSDFBrush, &ExtractedSDFOrder)> = brushes.iter().collect();
    brushes.sort_by(|a, b| a.1.order.cmp(&b.1.order));
    let brushes: Vec<Std140ExtractedSDFBrush> =
        brushes.iter().map(|val| val.0.as_std140()).collect();
    let num_brushes = brushes.len() as u64;

    brush_uniforms.settings.clear();
    brush_uniforms.settings.push(BrushSettings {
        num_brushes: num_brushes as i32,
    });
    brush_uniforms
        .settings
        .write_buffer(&render_device, &render_queue);

    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Brush Buffer"),
        contents: bytemuck::cast_slice(brushes.as_slice()),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    brush_uniforms.brushes = Some(buffer);
}

pub fn queue_brush_bindings(
    mut commands: Commands,
    buffers: Res<BrushUniforms>,
    render_device: Res<RenderDevice>,
    sdf_pipeline: Res<SDFPipeline>,
) {
    if let (Some(brushes), settings) = (&buffers.brushes, &buffers.settings) {
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
            ],
        });
        commands.spawn().insert(SDFBrushBinding {
            binding: brush_bind_group,
        });
    }
}

pub fn queue_sdf(
    mut commands: Commands,
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    sdf_pipeline: Res<SDFPipeline>,
    view_uniforms: Res<ViewUniforms>,
    view_extension_uniforms: Res<ViewExtensionUniforms>,
    render_device: Res<RenderDevice>,
    mut views: Query<(Entity, &ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_sdf = transparent_3d_draw_functions
        .read()
        .get_id::<DrawSDFCommand>()
        .unwrap();
    if let (Some(binding_resource), Some(extension_binding_resource)) = (
        view_uniforms.uniforms.binding(),
        view_extension_uniforms.uniforms.binding(),
    ) {
        for (entity, _view, mut transparent_phase) in views.iter_mut() {
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
            };
            commands.entity(entity).insert(view_binding);

            transparent_phase.add(Transparent3d {
                distance: 0.,
                pipeline: sdf_pipeline.pipeline,
                entity,
                draw_function: draw_sdf,
            });
        }
    }
}
