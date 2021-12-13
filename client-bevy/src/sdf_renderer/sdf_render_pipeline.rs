use bevy::{prelude::{Plugin, FromWorld, HandleUntyped, Assets}, render2::{render_resource::{CachedPipelineId, BindGroupLayout, RenderPipelineDescriptor, VertexState, VertexBufferLayout, FragmentState, RenderPipelineCache, Shader}, renderer::RenderDevice, texture::BevyDefault, mesh::{Mesh, shape}}, reflect::TypeUuid, math::Vec2};
use wgpu::{BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, BindingType, BufferBindingType, BufferSize, VertexAttribute, VertexFormat, VertexStepMode, PrimitiveState, FrontFace, Face, PolygonMode, PrimitiveTopology, DepthStencilState, TextureFormat, CompareFunction, StencilState, DepthBiasState, MultisampleState, ColorTargetState, BlendState, BlendComponent, BlendFactor, BlendOperation, ColorWrites};

use super::sdf_object_zones::ZoneSettings;

pub struct SDFRenderPipelinePlugin;

impl Plugin for SDFRenderPipelinePlugin {
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
            include_str!("shaders/general/zone_object_baked_bindings.wgsl"),
            include_str!("shaders/vertex/vertex_full_screen.wgsl"),
            include_str!("shaders/general/zone_object_baked_sdf_reader.wgsl"),
            include_str!("shaders/general/sdf_raymarch_use_secondary_hits.wgsl"),
            include_str!("shaders/fragment/full_fragment_secondary_hits.wgsl")
        ));
        shaders.set_untracked(SDF_SHADER_HANDLE, shader);
    }
}

pub struct SDFPipelineDefinitions {
    view_layout: BindGroupLayout,
    pipeline: CachedPipelineId,
}

pub const SDF_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745564647005696);
pub const SDF_CUBE_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 1674555646470534696);


impl FromWorld for SDFPipelineDefinitions {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();

        let zone_layout = {
            world.get_resource::<ZoneSettings>().unwrap().layout.clone()
        };

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
                zone_layout.clone(),
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
        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        Self {
            view_layout,
            pipeline: pipeline_cache.queue(descriptor),
        }
    }
}