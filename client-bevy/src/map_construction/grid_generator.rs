use std::collections::HashMap;

use bevy::{
    math::{Vec2, Vec3},
    prelude::{Entity, Plugin, Commands, Query, Changed, CoreStage, Parent, DespawnRecursiveExt, BuildChildren, Component, GlobalTransform},
};

use super::map_zones::{Zone, ZoneGrid, ZoneBoundary, DirtyZone, ZoneBrush, GetDistanceField, ZoneShape, ShapeOperation};

pub struct GridGeneratorPlugin;

impl Plugin for GridGeneratorPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_system_to_stage(CoreStage::Last, generate_points)
            .add_system_to_stage(CoreStage::Last, clear_old_grids);
    }
}

#[derive(Component)]
pub struct Grid {
    pub root_zone: Entity,
}

#[derive(Debug, Default, Clone)]
pub struct GridPoint {
    pub position: Vec2,
    pub zones: Vec<Entity>,
}

#[derive(Component)]
pub struct GridContents {
    pub points: Vec<GridPoint>,
}

pub struct GridZoneTriangulation {
    pub zone: Entity,
    pub verticies: Vec<Vec3>,
    pub indices: Vec<i32>,
}

fn clear_old_grids(mut commands: Commands, root_zones: Query<Entity, Changed<DirtyZone>>, grids: Query<(Entity, &Grid, &Parent)>) {
    grids.for_each(|(entity, _, parent)| {
       if let Ok(_) = root_zones.get(parent.0) {
           commands.entity(entity).despawn_recursive();
       }
    });
}

fn generate_points(mut commands: Commands, root_zones: Query<(Entity, &Zone, &ZoneGrid, &ZoneBoundary), Changed<DirtyZone>>, brushes: Query<(&GlobalTransform, &ZoneBrush, &Parent)>) {
    let mut zone_table = HashMap::<Entity, Vec<(f32, (GlobalTransform, ZoneShape, ShapeOperation))>>::new();
    root_zones.for_each(|(entity, _, _, _)| {
        zone_table.insert(entity, vec![]);
    });
    brushes.for_each(|(transform, brush, parent)| {
        if let Some(mut vec) = zone_table.get_mut(&parent.0){
            vec.push((brush.order, (transform.clone(), brush.shape.clone(), brush.operation.clone())));
        }
    });
    root_zones.for_each(|(entity, _, zone_grid, zone_boundary)| {
        if let Some(mut vec) = zone_table.get_mut(&entity) {
            vec.sort_by(|a,b| {
                a.0.partial_cmp(&b.0).unwrap()
            });
            let fill = generate_fill_points(entity, zone_grid, &vec);
            let boundary = generate_boundary_points(entity, zone_boundary, &vec);
            let full: Vec<GridPoint> = [fill, boundary].concat();
            let grid = commands.spawn().insert(Grid { root_zone: entity.clone()}).insert(GridContents { points: full }).id();
            commands.entity(entity).push_children(&[grid]);
        }
    });
}

fn generate_fill_points(zone: Entity, zone_settings: &ZoneGrid, brushes: &[(f32, (GlobalTransform, ZoneShape, ShapeOperation))]) -> Vec<GridPoint> {
    let mut vec = Vec::<GridPoint>::new();
    let bounds = brushes.into_iter().fold(None, |prev, brush| {
        brush.1.bounds(prev)
    });
    if let Some(bounds) = bounds {
        let boundary_size = zone_settings.grid_tile_size/2.;
        let mut x = (bounds.0.x + boundary_size);
        while x <= (bounds.1.x - boundary_size) {
            let mut y = (bounds.0.y + boundary_size);
            while y <=(bounds.1.y - boundary_size) {
                let point = Vec2::new(x,y);
                let dist = brushes.into_iter().fold(-5f32, |old, brush| {
                    brush.1.distance_field(point, old)
                });
                if (dist <= 0.) {
                    vec.push(GridPoint {
                        position: Vec2::new(x, y),
                        zones: vec![zone.clone()],
                    });
                }
                y += zone_settings.grid_tile_size;
            }
            
            x += zone_settings.grid_tile_size;
        }
    }
    vec
}

fn generate_boundary_points(zone: Entity, zone_settings: &ZoneBoundary, brushes: &[(f32, (GlobalTransform, ZoneShape, ShapeOperation))]) -> Vec<GridPoint> {
    vec![]
}

fn find_center_point(
    p1: Vec2,
    tile_size: f32,
    p1_dist: f32,
    p2_dist: f32,
    p3_dist: f32,
    p4_dist: f32,
) -> Option<(Vec2, Vec2)> {
    if p1_dist.signum() == p2_dist.signum()
        && p1_dist.signum() == p3_dist.signum()
        && p3_dist.signum() == p4_dist.signum()
    {
        return None;
    }
    let tile_size = tile_size.abs();
    let p2 = p1 + Vec2::X * tile_size;
    let p3 = p1 + Vec2::Y * tile_size;
    let p4 = p1 + Vec2::ONE * tile_size;

    let max_dist = p1_dist
        .abs()
        .max(p2_dist.abs().max(p3_dist.abs().max(p4_dist.abs())));
    let weights = (
        max_dist - p1_dist.abs(),
        max_dist - p2_dist.abs(),
        max_dist - p3_dist.abs(),
        max_dist - p4_dist.abs(),
    );
    let position = p1 * weights.0 + p2 * weights.1 + p3 * weights.2 + p4 * weights.3;
    let position = position / (weights.0 + weights.1 + weights.2 + weights.3);

    let normal_1 = Vec2::new(p2_dist - p1_dist, p3_dist - p1_dist) / tile_size;
    let normal_2 = Vec2::new(p4_dist - p2_dist, p4_dist - p3_dist) / tile_size;

    Some((position, ((normal_1 + normal_2) / 2.).normalize_or_zero()))
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_1_SQRT_2;

    use super::*;
    fn assert_eq_f32(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.00001
    }

    #[test]
    fn all_inside_has_no_point() {
        let result = find_center_point(Vec2::ZERO, 1., -1., -1., -1., -1.);
        assert!(result.is_none());
    }
    #[test]
    fn all_outside_has_no_point() {
        let result = find_center_point(Vec2::ZERO, 1., 1., 1., 1., 1.);
        assert!(result.is_none());
    }
    #[test]
    fn a_mix_of_inside_and_outside_has_a_point() {
        let result = find_center_point(Vec2::ZERO, 1., -1., 1., 1., 1.);
        assert!(result.is_some());
    }
    #[test]
    fn positions_the_point_with_average_distance() {
        let result = find_center_point(Vec2::ZERO, 1., -0.5, 0.5, 0.5, 1.06066);
        assert!(result.is_some());
        if let Some((position, normal)) = result {
            println!("position {} normal {}", position, normal);
            assert!(assert_eq_f32(position.x, 0.333_333_34));
            assert!(assert_eq_f32(position.y, 0.333_333_34));
            assert!(assert_eq_f32(normal.x, FRAC_1_SQRT_2));
            assert!(assert_eq_f32(normal.y, FRAC_1_SQRT_2));
        }
    }
}
