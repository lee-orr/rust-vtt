#![allow(clippy::many_single_char_names)]
use std::collections::HashMap;

use bevy::{
    math::{Vec2, Vec3, Vec4, Vec4Swizzles, Vec2Swizzles},
    prelude::{
        Bundle, Changed, Commands, Component, CoreStage, Entity, GlobalTransform, Or, Parent,
        Plugin, Query, Transform, RemovedComponents,
    },
};

pub struct MapZonePlugin;

impl Plugin for MapZonePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_to_stage(CoreStage::PostUpdate, mark_dirty_zone)
            .add_system_to_stage(CoreStage::PreUpdate, clear_dirty)
            .add_system_to_stage(CoreStage::PostUpdate, adjust_zone_hierarchy);
    }
}

#[derive(Component)]
pub struct ZoneHierarchy {
    pub zones: Vec<Entity>
}

#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ZoneOrderingId {
    pub order: u128,
}

impl ZoneOrderingId {
    fn from_zone_orders(order_in_layers: &[usize]) -> Option<ZoneOrderingId>{
        let mut order : u128 = 0;
        if order_in_layers.len() > 11 {
            return None;
        }
        for (layer, layer_order) in order_in_layers.iter().enumerate() {
            if *layer_order > 1000 {
                return None;
            }
            let power = (11 - layer) as u32;
            let multiplier = 1000u128.pow(power);
            order = multiplier * (*layer_order as u128) + order;
        }
        Some(ZoneOrderingId {
            order
        })
    }

    fn nearest_shared_zone(&self, b: &ZoneOrderingId) -> Option<ZoneOrderingId> {
        if self == b {
            Some(*self)
        } else {
            let mut shared : Option<ZoneOrderingId> = None;
            for layer in 1..12u32 {
                let divisor = 1000u128.pow(layer);
                let a_adjusted = self.order / divisor;
                let b_adjusted = b.order / divisor;
                if a_adjusted == b_adjusted {
                    shared = Some(ZoneOrderingId { order : a_adjusted * divisor});
                    break;
                }
            }
            shared
        }
    }
}

fn adjust_zone_hierarchy(
mut commands: Commands,
zones: Query<(Entity, &Zone, Option<&Parent>)>,
changed_zones: Query<(Entity, &Zone, &Parent), Changed<Parent>>
) {
    if changed_zones.is_empty() {
        return;
    }
    let mut hierarchy_map = HashMap::<Entity, Vec<Entity>>::new();
    for (entity, _, parent) in zones.iter() {
        if hierarchy_map.contains_key(&entity) {
            continue;
        }
        let mut descendents = Vec::<(Entity, &Zone, Option<&Parent>)>::new();
        let mut parent = parent;
        let mut last_entity = entity;
        let mut ancestors_till_now = Vec::<Entity>::new();
        descendents.insert(0, zones.get(entity).unwrap());
        while let Some(unwarpped_parent) = parent {
            if hierarchy_map.contains_key(unwarpped_parent) {
                ancestors_till_now = hierarchy_map.get(unwarpped_parent).unwrap().clone();
                ancestors_till_now.push(unwarpped_parent.0);
                break;
            }
            if let Ok(p) = zones.get(unwarpped_parent.0) {
                last_entity = unwarpped_parent.0;
                descendents.insert(0, p);
                parent = p.2;
            } else {
                break;
            }
        }
        if ancestors_till_now.len() == 0 {
            hierarchy_map.insert(last_entity, vec![]);
            commands.entity(last_entity).insert(ZoneHierarchy { zones: vec![]});
            ancestors_till_now.push(last_entity);
        }
        for (descendent, _, _) in descendents {
            hierarchy_map.insert(descendent, ancestors_till_now.clone());
            commands.entity(descendent).insert(ZoneHierarchy { zones: ancestors_till_now.clone()});
            ancestors_till_now.push(descendent);
        }
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DirtyZone;

fn mark_dirty_zone(
    mut commands: Commands,
    changed_brushes: Query<&ZoneBrush, Or<(Changed<ZoneBrush>, Changed<Transform>)>>,
    zones: Query<(Entity, &Zone, &Parent)>,
) {
    changed_brushes.for_each(|brush| {
        commands.entity(brush.zone).insert(DirtyZone);
        let mut child = brush.zone;
        while let Ok((_, _zone, parent)) = zones.get(child) {
            child = parent.0;
            commands.entity(child).insert(DirtyZone);
        }
    });
}

fn clear_dirty(mut commands: Commands, zones: Query<(Entity, &DirtyZone)>) {
    zones.for_each(|(entity, _)| {
        commands.entity(entity).remove::<DirtyZone>();
    });
}

#[derive(Debug, Clone, Copy)]
pub enum ZoneShape {
    Circle(f32),
    Square(f32, f32),
    Segment(Vec2, Vec2, f32),
    Curve(Vec2, Vec2, Vec2, f32),
}

impl ZoneShape {
    pub fn name(&self) -> &str {
        match self {
            ZoneShape::Circle(_) => "circle",
            ZoneShape::Square(_, _) => "square",
            ZoneShape::Segment(_, _, _) => "segment",
            ZoneShape::Curve(_, _, _, _) => "curve",
        }
    }

    pub fn distance_field(&self, point: Vec2) -> f32 {
        match self {
            ZoneShape::Circle(radius) => point.length() - *radius,
            ZoneShape::Square(width, height) => {
                let d = point.abs() - Vec2::new(*width / 2., *height / 2.);
                d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.)
            }
            ZoneShape::Segment(a, b, radius) => {
                let pa = point - *a;
                let ba = *b - *a;
                let h = (pa.dot(ba) / ba.dot(ba)).clamp(0., 1.);
                (pa - ba * h).length() - radius
            }
            ZoneShape::Curve(start, end, control, radius) => {
                let a = *control - *start;
                let b = *start - 2. * (*control) + *end;
                let c = a * 2.;
                let d = *start - point;

                let kk = 1. / b.dot(b);
                let kx = kk * a.dot(b);
                let ky = kk * (2. * a.dot(a) + d.dot(b)) / 3.;
                let kz = kk * d.dot(a);

                #[allow(unused_assignments)]
                let mut res = 0f32;

                let p = ky - kx * kx;
                let p3 = p * p * p;
                let q = kx * (2. * kx * kx - 3. * ky) + kz;
                let h = q * q + 4. * p3;

                if h >= 0. {
                    let h = h.sqrt();
                    let x = (Vec2::new(h, -h) - q) / 2.;
                    let uv = x.signum() * x.abs().powf(1. / 3.);
                    let t = (uv.x + uv.y - kx).clamp(0f32, 1f32);
                    let r = d + (c + b * t) * t;
                    res = r.dot(r);
                } else {
                    let z = (-p).sqrt();
                    let v = (q / (p * z * 2.)).acos() / 3.;
                    let m = v.cos();
                    let n = v.sin() * 1.732_050_8;
                    let t = (Vec3::new(m + m, -n - m, n - m) * z - kx).clamp(Vec3::ZERO, Vec3::ONE);
                    let dx = d + (c + b * t.x) * t.x;
                    let dy = d + (c + b * t.y) * t.y;
                    res = dx.dot(dx).min(dy.dot(dy));
                }
                res.sqrt() - radius
            }
        }
    }
    pub fn bounds(&self) -> (Vec2, Vec2) {
        match self {
            ZoneShape::Circle(radius) => (-*radius * Vec2::ONE, *radius * Vec2::ONE),
            ZoneShape::Square(width, height) => {
                let half = Vec2::new(*width, *height) / 2.;
                (-half, half)
            }
            ZoneShape::Segment(start, end, radius) => {
                (start.min(*end) - *radius, start.max(*end) + *radius)
            }
            ZoneShape::Curve(start, end, control, radius) => (
                start.min(end.min(*control)) - *radius,
                start.max(end.max(*control)) + *radius,
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ShapeOperation {
    Union,
    Subtraction,
}

impl ShapeOperation {
    pub fn name(&self) -> &str {
        match self {
            ShapeOperation::Union => "union",
            ShapeOperation::Subtraction => "subtraction",
        }
    }
    pub fn distance_field(&self, old: f32, next: f32) -> f32 {
        match self {
            ShapeOperation::Union => old.min(next),
            ShapeOperation::Subtraction => old.max(-next),
        }
    }

    pub fn bounds(&self, prev: (Vec2, Vec2), next: (Vec2, Vec2)) -> (Vec2, Vec2) {
        match self {
            ShapeOperation::Union => (prev.0.min(next.0), prev.1.max(next.1)),
            ShapeOperation::Subtraction => prev,
        }
    }
}

pub trait GetDistanceField {
    fn distance_field(&self, point: Vec2, old: f32) -> f32;
    fn bounds(&self, prev: Option<(Vec2, Vec2)>) -> Option<(Vec2, Vec2)>;
}

type ZoneShapeContainer = (GlobalTransform, ZoneShape, ShapeOperation);

impl GetDistanceField for ZoneShapeContainer {
    fn distance_field(&self, point: Vec2, old: f32) -> f32 {
        let (transform, shape, operation) = self;
        let matrix = transform.compute_matrix().inverse();
        let p = matrix * Vec4::new(point.x, 0., point.y, 1.);
        let point = p.xz();
        let next = shape.distance_field(point);
        operation.distance_field(old, next)
    }

    fn bounds(&self, prev: Option<(Vec2, Vec2)>) -> Option<(Vec2, Vec2)> {
        let (transform, shape, operation) = self;
        let next = shape.bounds();
        let matrix = transform.compute_matrix();
        let next = (
            matrix * Vec4::new(next.0.x, 0., next.0.y, 1.),
            matrix * Vec4::new(next.1.x, 0., next.1.y, 1.),
        );
        let next = (next.0.xz(), next.1.xz());
        let next = (next.0.min(next.1), next.0.max(next.1));
        if let Some(prev) = prev {
            Some(operation.bounds(prev, next))
        } else {
            Some(next)
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ZoneBrush {
    pub zone: Entity,
    pub shape: ZoneShape,
    pub operation: ShapeOperation,
    pub order: f32,
}

#[derive(Component, Debug, Default)]
pub struct Zone {
    pub name: String,
    pub order: u32
}

#[derive(Component, Debug)]
pub struct ZoneGrid {
    pub grid_noise: f32,
    pub alternative_grid: bool,
    pub grid_tile_size: f32,
}

impl Default for ZoneGrid {
    fn default() -> Self {
        Self {
            grid_noise: 1.,
            alternative_grid: false,
            grid_tile_size: 1.,
        }
    }
}

#[derive(Component, Debug)]
pub struct ZoneBoundary {
    pub boundary_noise: f32,
    pub boundary_width: f32,
}

impl Default for ZoneBoundary {
    fn default() -> Self {
        Self {
            boundary_noise: 0.,
            boundary_width: 0.1,
        }
    }
}

#[derive(Component, Debug)]
pub struct ZoneCeilingHeight {
    pub height: f32,
}

#[derive(Component, Debug)]
pub struct ZoneFloorHeight {
    pub height: f32,
}

#[derive(Component, Debug)]
pub struct ZoneWall {
    pub height: f32,
    pub width: f32,
}

#[derive(Bundle, Default)]
pub struct ZoneBundle {
    pub zone: Zone,
    pub grid: ZoneGrid,
    pub boundary: ZoneBoundary,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Bundle)]
pub struct BrushBundle {
    pub brush: ZoneBrush,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl BrushBundle {
    pub fn new(zone: Entity, siblings: i32) -> Self {
        Self {
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            brush: ZoneBrush {
                zone,
                shape: ZoneShape::Circle(1.),
                operation: ShapeOperation::Union,
                order: siblings as f32,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use bevy::{math::Quat, prelude::Transform};

    use super::*;
    fn assert_eq_f32(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.00001
    }

    #[test]
    fn circle_generates_correct_distances() {
        let circle = ZoneShape::Circle(1.);
        let center_dist = circle.distance_field(Vec2::ZERO);
        let border_dist = circle.distance_field(Vec2::X);
        let outside_dist = circle.distance_field(Vec2::X * 2.);

        assert!(assert_eq_f32(center_dist, -1.));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(outside_dist, 1.));
    }

    #[test]
    fn circle_generates_correct_bounds() {
        let circle = ZoneShape::Circle(1.);
        let bounds = circle.bounds();
        assert!(assert_eq_f32(bounds.0.x, -1.) && assert_eq_f32(bounds.0.y, -1.));
        assert!(assert_eq_f32(bounds.1.x, 1.) && assert_eq_f32(bounds.1.y, 1.));
    }

    #[test]
    fn square_generates_correct_distances() {
        let square = ZoneShape::Square(2., 2.);
        let center_dist = square.distance_field(Vec2::ZERO);
        let border_dist = square.distance_field(Vec2::ONE);
        let outside_dist = square.distance_field(Vec2::X * 2.);

        assert!(assert_eq_f32(center_dist, -1.));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(outside_dist, 1.));
    }

    #[test]
    fn square_generates_correct_bounds() {
        let square = ZoneShape::Square(1., 2.);
        let bounds = square.bounds();
        assert!(assert_eq_f32(bounds.0.x, -0.5) && assert_eq_f32(bounds.0.y, -1.));
        assert!(assert_eq_f32(bounds.1.x, 0.5) && assert_eq_f32(bounds.1.y, 1.));
    }

    #[test]
    fn segment_generates_correct_distance() {
        let segment = ZoneShape::Segment(Vec2::new(-1., 0.), Vec2::new(1., 1.), 1.);
        let center_dist = segment.distance_field(-1. * Vec2::X);
        let border_dist = segment.distance_field(-2. * Vec2::X);
        let border_dist_2 = segment.distance_field(Vec2::ONE + Vec2::X);
        let outside_dist = segment.distance_field(Vec2::ONE + 2. * Vec2::X);

        assert!(assert_eq_f32(center_dist, -1.));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(border_dist_2, 0.));
        assert!(assert_eq_f32(outside_dist, 1.));
    }

    #[test]
    fn segment_generates_correct_bounds() {
        let segment = ZoneShape::Segment(Vec2::new(-1., 0.), Vec2::new(1., 1.), 1.);
        let bounds = segment.bounds();
        assert!(assert_eq_f32(bounds.0.x, -2.) && assert_eq_f32(bounds.0.y, -1.));
        assert!(assert_eq_f32(bounds.1.x, 2.) && assert_eq_f32(bounds.1.y, 2.));
    }

    #[test]
    fn curve_generates_correct_distance() {
        let curve = ZoneShape::Curve(Vec2::new(-1., 0.), Vec2::new(1., 1.), Vec2::ZERO, 1.);
        let center_dist = curve.distance_field(-1. * Vec2::X);
        let border_dist = curve.distance_field(-2. * Vec2::X);
        let border_dist_2 = curve.distance_field(Vec2::ONE + Vec2::X);
        let outside_dist = curve.distance_field(Vec2::ONE + 2. * Vec2::X);

        assert!(assert_eq_f32(center_dist, -1.));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(border_dist_2, 0.));
        assert!(assert_eq_f32(outside_dist, 1.));
    }

    #[test]
    fn curve_generates_correct_bounds() {
        let curve = ZoneShape::Curve(Vec2::new(-1., 0.), Vec2::new(1., 1.), Vec2::ZERO, 1.);
        let bounds = curve.bounds();
        assert!(assert_eq_f32(bounds.0.x, -2.) && assert_eq_f32(bounds.0.y, -1.));
        assert!(assert_eq_f32(bounds.1.x, 2.) && assert_eq_f32(bounds.1.y, 2.));
    }

    #[test]
    fn union_generates_correct_distance() {
        let union = ShapeOperation::Union;
        let result_a = union.distance_field(-1., 2.);
        let result_b = union.distance_field(2., -1.);
        assert!(assert_eq_f32(result_a, -1.));
        assert!(assert_eq_f32(result_b, -1.));
    }

    #[test]
    fn union_generates_correct_bounds() {
        let union = ShapeOperation::Union;
        let bounds = union.bounds((-3. * Vec2::ONE, Vec2::X), (-4. * Vec2::Y, Vec2::ZERO));
        assert!(assert_eq_f32(bounds.0.x, -3.) && assert_eq_f32(bounds.0.y, -4.));
        assert!(assert_eq_f32(bounds.1.x, 1.) && assert_eq_f32(bounds.1.y, 0.));
    }

    #[test]
    fn subtraction_generates_correct_distance() {
        let subtraction = ShapeOperation::Subtraction;
        let result_a = subtraction.distance_field(-1., 2.);
        let result_b = subtraction.distance_field(2., -1.);
        let result_c = subtraction.distance_field(-1., -2.);
        assert!(assert_eq_f32(result_a, -1.));
        assert!(assert_eq_f32(result_b, 2.));
        assert!(assert_eq_f32(result_c, 2.));
    }

    #[test]
    fn subtraction_generates_correct_bounds() {
        let subtraction = ShapeOperation::Subtraction;
        let bounds = subtraction.bounds((-3. * Vec2::ONE, Vec2::X), (-4. * Vec2::Y, Vec2::ZERO));
        assert!(assert_eq_f32(bounds.0.x, -3.) && assert_eq_f32(bounds.0.y, -3.));
        assert!(assert_eq_f32(bounds.1.x, 1.) && assert_eq_f32(bounds.1.y, 0.));
    }

    #[test]
    fn full_operations_generate_correct_distance() {
        let transform =
            Transform::from_xyz(1., 0., 0.).with_rotation(Quat::from_rotation_y(PI / 2.));

        let operations = (
            GlobalTransform::from(transform),
            ZoneShape::Square(2., 1.),
            ShapeOperation::Union,
        );
        let center_dist = operations.distance_field(Vec2::X, 0.5);
        let border_dist = operations.distance_field(Vec2::X * 0.5, 0.5);
        let border_dist_2 = operations.distance_field(Vec2::new(1.5, 1.), 0.5);
        let outside_dist = operations.distance_field(Vec2::ZERO, 0.5);

        assert!(assert_eq_f32(center_dist, -0.5));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(border_dist_2, 0.));
        assert!(assert_eq_f32(outside_dist, 0.5));
    }

    #[test]
    fn full_operations_generate_correct_bounds() {
        let transform =
            Transform::from_xyz(1., 0., 0.).with_rotation(Quat::from_rotation_y(PI / 2.));

        let operations = (
            GlobalTransform::from(transform),
            ZoneShape::Square(2., 1.),
            ShapeOperation::Union,
        );
        let bounds = operations
            .bounds(Some((-3. * Vec2::ONE, Vec2::ZERO)))
            .unwrap();
        assert!(assert_eq_f32(bounds.0.x, -3.) && assert_eq_f32(bounds.0.y, -3.));
        assert!(assert_eq_f32(bounds.1.x, 1.5) && assert_eq_f32(bounds.1.y, 1.));
    }

    #[test]
    fn generate_correct_zone_ordering_id() {
        let order = [13, 5, 6];
        let ordering = ZoneOrderingId::from_zone_orders(&order);
        assert!(ordering.is_some());
        assert_eq!(ordering.unwrap().order, 13005006000000000000000000000000000u128);
    }

    #[test]
    fn get_correct_shared_zone_id() {
        let a = ZoneOrderingId::from_zone_orders(&[12, 5, 6]).unwrap();
        let b = ZoneOrderingId::from_zone_orders(&[12, 7, 10]).unwrap();
        let shared_expected = ZoneOrderingId::from_zone_orders(&[12]).unwrap();
        let shared_result = a.nearest_shared_zone(&b);
        assert!(shared_result.is_some());
        assert_eq!(shared_result.unwrap(), shared_expected);
    }
}
