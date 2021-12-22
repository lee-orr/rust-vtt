use std::collections::HashMap;

use bevy::{
    math::Vec3,
    prelude::{Commands, Component, Entity, FromWorld, Plugin, Query, Res},
    render::{
        render_resource::{BindGroup, BindGroupLayout},
        renderer::RenderDevice,
        RenderApp, RenderStage,
    },
};
use crevice::std140::AsStd140;
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, ShaderStages,
};

use super::{sdf_operation::SDFGlobalNodeBounds, sdf_origin::SDFOrigin};

pub struct SDFObjectZonePlugin;

impl Plugin for SDFObjectZonePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app(RenderApp)
            .init_resource::<ZoneSettings>()
            .add_system_to_stage(RenderStage::Prepare, prepare_zones);
    }
}

#[derive(Component, Clone)]
pub struct SDFZones {
    pub zone_group: BindGroup,
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
            size: 300.,
            zones_per_dimension: 30,
        }
    }
}

#[derive(AsStd140)]
struct ZoneBoundSettings {
    num_zones: i32,
    zone_radius: f32,
    zone_size: Vec3,
    zone_half_size: Vec3,
    zone_origin: Vec3,
    zones_per_dimension: i32,
    world_center: Vec3,
    world_bounds: Vec3,
}

#[derive(Clone, Debug, Copy, AsStd140, Default)]
pub struct SDFZoneDefinitions {
    pub min: Vec3,
    pub max: Vec3,
    pub center: Vec3,
    pub first_object: i32,
    pub final_object: i32,
}

fn process_object_zones(
    objects: &[(Entity, &SDFGlobalNodeBounds)],
    size: f32,
    zones_per_dimension: i32,
    origin: &SDFOrigin,
) -> (Vec<i32>, Vec<SDFZoneDefinitions>, usize, f32, Vec3, Vec3) {
    let mut zone_objects: Vec<i32> = Vec::new();
    let mut active_zones: Vec<SDFZoneDefinitions> = Vec::new();
    let mut zone_hash: HashMap<(u32, u32, u32), Vec<i32>> = HashMap::new();
    let zone_size = Vec3::ONE * (size / (zones_per_dimension as f32)).abs();
    let zone_half_size = zone_size / 2.;
    let zone_radius = zone_half_size.length();
    let bounds_min = origin.origin - (size / 2.);

    for (obj, (_, bounds)) in objects.iter().enumerate() {
        let zone_bound_radius = bounds.radius;
        let min_zone_bound = -bounds.center - zone_bound_radius;
        let max_zone_bound = -bounds.center + zone_bound_radius;
        let min_zone_bound = ((min_zone_bound - bounds_min) / zone_size).floor();
        let max_zone_bound = ((max_zone_bound - bounds_min) / zone_size).ceil();
        for x in (min_zone_bound.x as i32)..(max_zone_bound.x as i32) {
            if x > zones_per_dimension {
                break;
            }
            if x < 0 {
                continue;
            }
            for y in (min_zone_bound.y as i32)..(max_zone_bound.y as i32) {
                if y > zones_per_dimension {
                    break;
                }

                if y < 0 {
                    continue;
                }
                for z in (min_zone_bound.z as i32)..(max_zone_bound.z as i32) {
                    if z > zones_per_dimension {
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

    for x in 0..zones_per_dimension {
        for y in 0..zones_per_dimension {
            for z in 0..zones_per_dimension {
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
                    center: min + zone_half_size,
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
        active_zones.push(SDFZoneDefinitions::default());
    }
    (
        zone_objects,
        active_zones,
        num_zones,
        zone_radius,
        zone_size,
        bounds_min,
    )
}

fn prepare_zones(
    mut commands: Commands,
    query: Query<(Entity, &SDFGlobalNodeBounds)>,
    render_device: Res<RenderDevice>,
    settings: Res<ZoneSettings>,
    origin: Res<SDFOrigin>,
) {
    let mut objects = query.iter().collect::<Vec<_>>();

    objects.sort_by(|a, b| a.0.cmp(&b.0));

    let (zone_objects, active_zones, num_zones, zone_radius, zone_size, bounds_min) =
        process_object_zones(
            &objects,
            settings.size,
            settings.zones_per_dimension,
            &origin,
        );

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

    let num_zones = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Num Zones"),
        contents: bytemuck::cast_slice(&[(ZoneBoundSettings {
            num_zones: num_zones as i32,
            zone_radius,
            zone_size,
            zone_half_size: zone_size / 2.,
            zone_origin: bounds_min,
            zones_per_dimension: settings.zones_per_dimension as i32,
            world_bounds: (settings.size / 2.) * Vec3::ONE,
            world_center: origin.origin,
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

    let zones = SDFZones {
        zone_group: bind_group,
    };
    commands.spawn().insert(zones);
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn with_no_objects_all_zones_are_empty() {
        let objects = Vec::new();
        let result = process_object_zones(&objects, 10., 3, &SDFOrigin { origin: Vec3::ZERO });
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.2, 27);
        assert_eq!(result.1.len(), result.2);
        for zone in result.1.iter() {
            assert_eq!(zone.final_object, zone.first_object);
        }
    }

    #[test]
    fn with_one_object_only_containing_zone_has_references() {
        let objects = vec![(
            Entity::new(0),
            &SDFGlobalNodeBounds {
                center: Vec3::ZERO,
                radius: 0.2,
            },
        )];
        let result = process_object_zones(&objects, 10., 3, &SDFOrigin { origin: Vec3::ZERO });
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.2, 27);
        assert_eq!(result.1.len(), result.2);
        let contain_item = [13];
        for (i, zone) in result.1.iter().enumerate() {
            println!("{} {} {}", i, zone.final_object, zone.first_object);
            if contain_item.contains(&i) {
                assert_eq!(zone.final_object, zone.first_object + 1);
            } else {
                assert_eq!(zone.final_object, zone.first_object);
            }
        }
    }

    #[test]
    fn with_two_objects_only_containing_zone_has_references() {
        let second_pos = SDFGlobalNodeBounds {
            center: Vec3::new(0., 4., 0.),
            radius: 0.2,
        };
        let objects = vec![
            (
                Entity::new(0),
                &SDFGlobalNodeBounds {
                    center: Vec3::ZERO,
                    radius: 0.2,
                },
            ),
            (Entity::new(1), &second_pos),
        ];
        let result = process_object_zones(&objects, 10., 3, &SDFOrigin { origin: Vec3::ZERO });
        assert_eq!(result.0.len(), 2);
        assert_eq!(result.2, 27);
        assert_eq!(result.1.len(), result.2);
        let contain_item = [13, 16];
        for (i, zone) in result.1.iter().enumerate() {
            println!("{} {} {}", i, zone.final_object, zone.first_object);
            if contain_item.contains(&i) {
                assert_eq!(zone.final_object, zone.first_object + 1);
            } else {
                assert_eq!(zone.final_object, zone.first_object);
            }
        }
    }

    #[test]
    fn witn_only_one_object_and_a_non_centered_origin_only_containing_zones_have_object() {
        let objects = vec![(
            Entity::new(0),
            &SDFGlobalNodeBounds {
                center: Vec3::ZERO,
                radius: 0.2,
            },
        )];
        let result = process_object_zones(
            &objects,
            10.,
            3,
            &SDFOrigin {
                origin: Vec3::new(0., -4., 0.),
            },
        );
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.2, 27);
        assert_eq!(result.1.len(), result.2);
        let contain_item = [16];
        for (i, zone) in result.1.iter().enumerate() {
            println!("{} {} {}", i, zone.final_object, zone.first_object);
            if contain_item.contains(&i) {
                assert_eq!(zone.final_object, zone.first_object + 1);
            } else {
                assert_eq!(zone.final_object, zone.first_object);
            }
        }
    }
}
