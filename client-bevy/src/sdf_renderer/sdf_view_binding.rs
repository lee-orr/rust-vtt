use bevy::{
    prelude::*,
    render::{
        camera::PerspectiveProjection,
        render_resource::{BindGroup, BindGroupLayout, DynamicUniformVec},
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, ViewUniforms},
        RenderApp, RenderStage,
    },
};
use crevice::std140::AsStd140;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, BufferSize, ShaderStages,
};

pub struct SDFViewBindingPlugin;

impl Plugin for SDFViewBindingPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFViewLayout>()
            .init_resource::<ViewExtensionUniforms>()
            .add_system_to_stage(RenderStage::Prepare, prepare_view_extensions)
            .add_system_to_stage(RenderStage::Queue, queue_view_bindings);
    }
}

pub struct SDFViewLayout {
    pub layout: BindGroupLayout,
}

impl FromWorld for SDFViewLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF View Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
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
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
            ],
        });
        Self { layout }
    }
}

#[derive(Component)]
pub struct SDFViewBinding {
    pub binding: BindGroup,
}

#[derive(Clone, AsStd140, Component)]
pub struct ViewExtension {
    view_proj_inverted: Mat4,
    proj_inverted: Mat4,
    cone_scaler: f32,
    pixel_size: f32,
    near: f32,
    far: f32,
}

#[derive(Default, Component)]
pub struct ViewExtensionUniforms {
    pub uniforms: DynamicUniformVec<ViewExtension>,
}

#[derive(Component)]
pub struct ViewExtensionUniformOffset {
    pub offset: u32,
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
                near: camera.near,
                far: camera.far
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

fn queue_view_bindings(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    view_extension_uniforms: Res<ViewExtensionUniforms>,
    view_layout: Res<SDFViewLayout>,
) {
    if let (Some(binding_resource), Some(extension_binding_resource)) = (
        view_uniforms.uniforms.binding(),
        view_extension_uniforms.uniforms.binding(),
    ) {
        let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("View Bind Group"),
            layout: &view_layout.layout,
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
        commands.spawn().insert(view_binding);
    }
}
