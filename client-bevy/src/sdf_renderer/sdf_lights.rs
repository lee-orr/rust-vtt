use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroup, BindGroupLayout, DynamicUniformVec},
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
};
use crevice::std140::AsStd140;
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, BufferUsages, ShaderStages,
};



pub struct SDFLightPlugin;

impl Plugin for SDFLightPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFLightBindingLayout>()
            .add_system_to_stage(RenderStage::Extract, extract_lights)
            .add_system_to_stage(RenderStage::Queue, queue_light_bindings);
    }
}

#[derive(Component)]
pub struct SDFPointLight {
    pub distance: f32,
    pub color: Color,
}

#[derive(Component)]
pub struct SDFDirectionalLight {
    pub color: Color,
}

#[derive(Component)]
pub struct SDFSpotLight {
    pub distance: f32,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub color: Color,
}

pub struct SDFLightBindingLayout {
    pub layout: BindGroupLayout,
}

impl FromWorld for SDFLightBindingLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Pipeline Brush Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
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
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
            ],
        });
        Self { layout }
    }
}

#[derive(Component)]
pub struct LightBindingGroup {
    pub binding: BindGroup,
}

#[derive(Component, Clone, Debug, Copy, AsStd140, Default)]
pub struct GPULight {
    pub color: Vec4,
    pub params: Mat4,
}

fn extract_lights(
    mut commands: Commands,
    point_lights: Query<(Entity, &GlobalTransform, &SDFPointLight)>,
    spot_lights: Query<(Entity, &GlobalTransform, &SDFSpotLight)>,
    directional_lights: Query<(Entity, &GlobalTransform, &SDFDirectionalLight)>,
) {
    for (entity, transform, light) in point_lights.iter() {
        commands.get_or_spawn(entity).insert(GPULight {
            color: light.color.as_rgba_f32().into(),
            params: Mat4::from_cols(
                Vec4::new(
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                    0.,
                ),
                Vec4::ZERO,
                Vec4::new(light.distance, 2. * PI, 2. * PI, 0.),
                Vec4::ZERO,
            ),
        });
    }

    for (entity, transform, light) in spot_lights.iter() {
        commands.get_or_spawn(entity).insert(GPULight {
            color: light.color.as_rgba_f32().into(),
            params: Mat4::from_cols(
                Vec4::new(
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                    1.,
                ),
                transform.rotation.into(),
                Vec4::new(light.distance, light.inner_radius, light.outer_radius, 0.),
                Vec4::ZERO,
            ),
        });
    }

    for (entity, transform, light) in directional_lights.iter() {
        commands.get_or_spawn(entity).insert(GPULight {
            color: light.color.as_rgba_f32().into(),
            params: Mat4::from_cols(
                Vec4::W * 2.,
                transform.rotation.into(),
                Vec4::ZERO,
                Vec4::ZERO,
            ),
        });
    }
}

pub fn queue_light_bindings(
    mut commands: Commands,
    lights: Query<&GPULight>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    bind_layout: Res<SDFLightBindingLayout>,
) {
    let mut lights = lights.iter().map(|v| v.as_std140()).collect::<Vec<_>>();
    let mut count = DynamicUniformVec::<i32>::default();
    count.clear();
    count.push(lights.len() as i32);
    count.write_buffer(&render_device, &render_queue);
    if lights.is_empty() {
        lights.push(GPULight::default().as_std140());
    }

    let lights = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Light Buffer"),
        contents: bytemuck::cast_slice(lights.as_slice()),
        usage: BufferUsages::STORAGE,
    });

    let light_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("Light Bind Group"),
        layout: &bind_layout.layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: lights.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: count.binding().unwrap(),
            },
        ],
    });
    let binding = LightBindingGroup {
        binding: light_bind_group,
    };
    commands.spawn().insert(binding);
}
