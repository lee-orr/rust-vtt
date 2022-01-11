use bevy::{
    math::{Vec2, Vec3},
    prelude::{Entity, Plugin},
};

pub struct GridGeneratorPlugin;

impl Plugin for GridGeneratorPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

pub struct Grid {
    pub level: i32,
    pub root_zone: Entity,
}

pub struct GridPoint {
    pub position: Vec2,
    pub zones: Vec<Entity>,
}

pub struct GridContents {
    pub points: Vec<GridPoint>,
}

pub struct GridZoneTriangulation {
    pub zone: Entity,
    pub verticies: Vec<Vec3>,
    pub indices: Vec<i32>,
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
