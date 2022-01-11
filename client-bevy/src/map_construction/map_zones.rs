use bevy::{
    math::{Vec2, Vec3, Vec4, Vec4Swizzles},
    prelude::{Component, Entity, GlobalTransform},
};

#[derive(Debug)]
pub enum ZoneShape {
    Circle(f32),
    Square(f32, f32),
    Segment(Vec2, Vec2, f32),
    Curve(Vec2, Vec2, Vec2, f32),
}

impl ZoneShape {
    pub fn distance_field(&self, point: Vec2) -> f32 {
        match self {
            ZoneShape::Circle(radius) => point.length() - *radius,
            ZoneShape::Square(width, height) => {
                let d = point.abs() - Vec2::new(*width, *height);
                d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.)
            }
            ZoneShape::Segment(a, b, radius) => {
                let pa = point - *a;
                let ba = *b - *a;
                let h = (pa.dot(ba) / ba.dot(ba)).clamp(0., 1.);
                (pa - ba * h).length() - radius
            }
            ZoneShape::Curve(start, control, end, radius) => {
                let a = *end - *start;
                let b = *start - 2. * (*end) + *control;
                let c = a * 2.;
                let d = *start - point;

                let kk = 1. / b.dot(b);
                let kx = kk * a.dot(b);
                let ky = kk * (2. * a.dot(a) + d.dot(b)) / 3.;
                let kz = kk * d.dot(a);

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
}

#[derive(Debug)]
pub enum ShapeOperation {
    Union,
    Subtraction,
}

impl ShapeOperation {
    pub fn distance_field(&self, old: f32, next: f32) -> f32 {
        match self {
            ShapeOperation::Union => old.min(next),
            ShapeOperation::Subtraction => old.max(-next),
        }
    }
}

pub trait GetDistanceField {
    fn distance_field(&self, point: Vec2, old: f32) -> f32;
}

type ZoneShapeContainer = (GlobalTransform, ZoneShape, ShapeOperation);

impl GetDistanceField for ZoneShapeContainer {
    fn distance_field(&self, point: Vec2, old: f32) -> f32 {
        let (transform, shape, operation) = self;
        let matrix = transform.compute_matrix();
        let p = matrix * Vec4::new(point.x, 0., point.y, 1.);
        let point = p.xz();
        let next = shape.distance_field(point);
        operation.distance_field(old, next)
    }
}

#[derive(Component, Debug)]
pub struct ZoneBrush {
    pub zone: Entity,
    pub shape: ZoneShape,
    pub operation: ShapeOperation,
    pub order: f32,
}

#[derive(Component, Debug)]
pub struct Zone {}

#[derive(Component, Debug)]
pub struct ZoneGrid {
    pub grid_noize: f32,
    pub alternative_grid: bool,
    pub grid_tile_size: f32,
}

#[derive(Component, Debug)]
pub struct ZoneBoundary {
    pub boundary_noise: f32,
    pub boundary_width: f32,
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

#[cfg(test)]
mod tests {
    use super::*;
    fn assert_eq_f32(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
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
    fn square_generates_correct_distances() {
        let square = ZoneShape::Square(1., 1.);
        let center_dist = square.distance_field(Vec2::ZERO);
        let border_dist = square.distance_field(Vec2::ONE);
        let outside_dist = square.distance_field(Vec2::X * 2.);

        assert!(assert_eq_f32(center_dist, -1.));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(outside_dist, 1.));
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
    fn curve_generates_correct_distance() {
        let curve = ZoneShape::Curve(Vec2::new(-1., 0.), Vec2::new(1., 1.), Vec2::ZERO, 1.);
        let center_dist = curve.distance_field(-1. * Vec2::X);
        let border_dist = curve.distance_field(-2. * Vec2::X);
        let border_dist_2 = curve.distance_field(Vec2::ONE + Vec2::X);
        let outside_dist = curve.distance_field(Vec2::ONE + 2. * Vec2::X);

        println!(
            "{}, {}, {}, {}",
            center_dist, border_dist, border_dist_2, outside_dist
        );

        assert!(assert_eq_f32(center_dist, -1.));
        assert!(assert_eq_f32(border_dist, 0.));
        assert!(assert_eq_f32(border_dist_2, 0.));
        assert!(assert_eq_f32(outside_dist, 1.));
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
    fn subtraction_generates_correct_distance() {
        let subtraction = ShapeOperation::Subtraction;
        let result_a = subtraction.distance_field(-1., 2.);
        let result_b = subtraction.distance_field(2., -1.);
        let result_c = subtraction.distance_field(-1., -2.);
        assert!(assert_eq_f32(result_a, -1.));
        assert!(assert_eq_f32(result_b, 2.));
        assert!(assert_eq_f32(result_c, 2.));
    }
}
