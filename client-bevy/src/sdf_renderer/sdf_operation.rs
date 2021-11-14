use bevy::{ecs::system::Command, math::{Mat4, Quat, Vec3, Vec4}, prelude::{Commands, Entity, GlobalTransform, Query}, render2::render_resource::DynamicUniformVec};
use bytemuck::{Pod, Zeroable};
use crevice::std140::AsStd140;

#[repr(C)]
#[derive(Clone, Copy, Default, AsStd140, Debug)]
pub struct ExtractedSDFBrush {
    shape: i32,
    operation: i32,
    blending: f32,
    transform: Mat4,
    param1: Vec4,
    param2: Vec4
}

#[derive(Debug)]
pub struct ExtractedSDFOrder {
    pub order: u32,
}

#[derive(Default, Clone, AsStd140)]
pub struct BrushSettings {
    pub num_brushes: i32,
}

pub enum SDFShape {
    Sphere(f32),
    Box(f32, f32, f32),
}

pub enum SDFOperation {
    Union,
    Subtraction,
    Intersection,
}

pub struct SDFBrush {
    pub order: u32,
    pub shape: SDFShape,
    pub operation: SDFOperation,
    pub blending: f32,
}

const SPHERE_CODE : i32 = 0;
const SQUARE_CODE : i32 = 1;

const UNION_CODE : i32 = 0;
const SUBTRACTION_CODE: i32 = 1;
const INTERSECTION_CODE: i32 = 2;

fn extract_sdf_brush(transform: &GlobalTransform, brush: &SDFBrush) -> (ExtractedSDFBrush, ExtractedSDFOrder) {
    let mut extracted = match brush.shape {
        SDFShape::Sphere(radius) => ExtractedSDFBrush { shape: SPHERE_CODE, param1: Vec4::new(radius, 0., 0., 0.), ..Default::default()},
        SDFShape::Box(width, height, depth) => ExtractedSDFBrush{ shape: SQUARE_CODE, param1: Vec4::new(width, height, depth, 0.), ..Default::default()},
    };
    extracted.transform = transform.compute_matrix();
    extracted.blending = brush.blending;
    extracted.operation = match brush.operation {
        SDFOperation::Union => UNION_CODE,
        SDFOperation::Subtraction => SUBTRACTION_CODE,
        SDFOperation::Intersection => INTERSECTION_CODE,
    };
    (extracted, ExtractedSDFOrder { order: brush.order })
}

pub fn extract_sdf_brushes(mut commands: Commands, brushes: Query<(Entity, &GlobalTransform, &SDFBrush)>) {
    for (entity, transform, brush) in brushes.iter() {
        let (sdf, order) = extract_sdf_brush(&transform, &brush);
        commands.get_or_spawn(entity).insert(sdf).insert(order);
    }
}