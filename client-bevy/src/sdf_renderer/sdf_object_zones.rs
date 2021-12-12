use std::collections::HashMap;

use bevy::{prelude::{Plugin, Component, Commands, Entity, Query, Res, ResMut, FromWorld}, render2::{RenderApp, render_resource::{BindGroup, BindGroupLayout}, RenderStage, renderer::RenderDevice}, math::Vec3};
use crevice::std140::AsStd140;
use wgpu::{util::BufferInitDescriptor, BufferUsages, BindGroupEntry, BindGroupDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, ShaderStages, BindGroupLayoutDescriptor};

use super::{sdf_operation::SDFGlobalNodeBounds, sdf_baker::{SDFBakerSettings, SDFBakedLayerOrigins, SDFBakerPipelineDefinitions}};

pub struct SDFObjectZonePlugin;

impl Plugin for SDFObjectZonePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app(RenderApp)
            .init_resource::<SDFZones>()
            .init_resource::<ZoneSettings>()
            .add_system_to_stage(RenderStage::Prepare, prepare_zones);
    }
}


#[derive(Default, Component, Clone)]
pub struct SDFZones {
    pub zone_group: Option<BindGroup>,
}

pub struct ZoneSettings {
    pub layout: BindGroupLayout,
    pub size: f32,
    pub zones_per_dimension: i32,
}

impl FromWorld for ZoneSettings {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();
        let device = world.get_resource::<RenderDevice>().unwrap();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Zone Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
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
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
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
                    binding: 2,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        Self {
            layout,
            size: 100.,
            zones_per_dimension: 10,
        }
    }
}

#[derive(AsStd140)]
struct NumZones {
    num_zones: i32,
    zone_radius: f32,
    zone_size: Vec3,
    zone_origin: Vec3,
    zones_per_dimension: i32,
}

#[derive(Clone, Debug, Copy, AsStd140, Default)]
pub struct SDFZoneDefinitions {
    pub min: Vec3,
    pub max: Vec3,
    pub first_object: i32,
    pub final_object: i32,
}

fn prepare_zones(
    mut commands: Commands,
    query: Query<(Entity, &SDFGlobalNodeBounds)>,
    render_device: Res<RenderDevice>,
    settings: Res<ZoneSettings>,
    origin: Res<SDFBakedLayerOrigins>,
    sdf_pipeline: Res<SDFBakerPipelineDefinitions>,
) {
    let mut zones = SDFZones::default();
    let mut objects = query.iter().collect::<Vec<_>>();

    objects.sort_by(|a, b| a.0.cmp(&b.0));

    let mut zone_objects: Vec<i32> = Vec::new();
    let mut active_zones: Vec<SDFZoneDefinitions> = Vec::new();
    let mut zone_hash: HashMap<(u32, u32, u32), Vec<i32>> = HashMap::new();
    let zone_size = Vec3::ONE * (settings.size / (settings.zones_per_dimension as f32)).abs();
    let zone_half_size = zone_size / 2.;
    let zone_radius = zone_half_size.length();
    let bounds_min = origin.origin - (settings.size / 2.);
    let effective_radius = zone_radius;

    for (obj, (_, bounds)) in objects.iter().enumerate() {
        let zone_bound_radius = bounds.radius + effective_radius;
        let min_zone_bound = bounds.center - zone_bound_radius;
        let max_zone_bound = bounds.center + zone_bound_radius;
        let min_zone_bound = ((min_zone_bound - bounds_min) / zone_size).floor();
        let max_zone_bound = ((max_zone_bound - bounds_min) / zone_size).floor();
        for x in (min_zone_bound.x as i32)..(max_zone_bound.x as i32) {
            if x > settings.zones_per_dimension {
                break;
            }
            if x < 0 {
                continue;
            }
            for y in (min_zone_bound.y as i32)..(max_zone_bound.y as i32) {
                if y > settings.zones_per_dimension {
                    break;
                }

                if y < 0 {
                    continue;
                }
                for z in (min_zone_bound.z as i32)..(max_zone_bound.z as i32) {
                    if z > settings.zones_per_dimension {
                        break;
                    }
                    if z < 0 {
                        continue;
                    }
                    let key = (x as u32, y as u32, z as u32);
                    zone_hash.entry(key).or_insert_with(Vec::new);
                    let vec = zone_hash.get_mut(&key);
                    if let Some(vec) = vec {
                        vec.push(obj as i32);
                    }
                }
            }
        }
    }

    for x in 0..settings.zones_per_dimension {
        for y in 0..settings.zones_per_dimension {
            for z in 0..settings.zones_per_dimension {
                let zone_objects_vec = zone_hash.get_mut(&(x as u32, y as u32, z as u32));
                let offset = Vec3::new(x as f32, y as f32, z as f32);
                let min = offset * zone_size + bounds_min;
                let max = min + zone_size;
                let (first_object, final_object) = match zone_objects_vec {
                    Some(vec) => {
                        let a = zone_objects.len() as i32;
                        zone_objects.append(vec);
                        let b = zone_objects.len() as i32;
                        (a, b)
                    }
                    None => (0, 0),
                };
                let zone = SDFZoneDefinitions {
                    min,
                    max,
                    first_object,
                    final_object,
                };
                active_zones.push(zone);
            }
        }
    }

    if zone_objects.is_empty() {
        println!("Zone objects empty");
        zone_objects.push(0);
    }

    let num_zones = active_zones.len();
    if num_zones == 0 {
        active_zones.push(SDFZoneDefinitions::default())
    }

    let zone_object_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Zone Objects"),
        contents: bytemuck::cast_slice(
            zone_objects
                .iter()
                .map(|a| a.as_std140())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        usage: BufferUsages::STORAGE,
    });

    let zone_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Zones"),
        contents: bytemuck::cast_slice(
            active_zones
                .iter()
                .map(|a| a.as_std140())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        usage: BufferUsages::STORAGE,
    });

    // println!("Num Zones: {}", num_zones);

    let num_zones = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Num Zones"),
        contents: bytemuck::cast_slice(&[(NumZones {
            num_zones: num_zones as i32,
            zone_radius,
            zone_size,
            zone_origin: bounds_min,
            zones_per_dimension: settings.zones_per_dimension as i32,
        })
        .as_std140()]),
        usage: BufferUsages::UNIFORM,
    });

    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("Zone Bind Group"),
        layout: &settings.layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: zone_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: zone_object_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: num_zones.as_entire_binding(),
            },
        ],
    });
    zones.zone_group = Some(bind_group);
    commands.spawn().insert(zones.clone());
    commands.insert_resource(zones);
}