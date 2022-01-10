use bevy::math::{Vec2, Vec3};

pub trait DistanceFunction2D {
    fn distance_function(&self, point: Vec2) -> f32;
}

pub trait DistanceFunction3D {
    fn distance_function(&self, point: Vec3) -> (f32, Vec3);
}

pub enum Shape2D {
    Circle(f32),
    Rectangle(f32, f32),
}

impl DistanceFunction2D for Shape2D {
    fn distance_function(&self, point: Vec2) -> f32 {
        match self {
            Shape2D::Circle(radius) => point.length() - radius,
            Shape2D::Rectangle(width, height) => {
                let d = point.abs() - Vec2::new(*width, *height);
                (d.max(Vec2::ZERO) + d.x.max(d.y).min(0.)).length()
            }
        }
    }
}

pub enum Shape3D {
    Box(f32, f32, f32),
    Cylinder(f32, f32),
}

impl DistanceFunction3D for Shape3D {
    fn distance_function(&self, point: Vec3) -> (f32, Vec3) {
        match self {
            Shape3D::Box(_, _, _) => todo!(),
            Shape3D::Cylinder(_, _) => todo!(),
        }
    }
}