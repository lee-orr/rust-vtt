use bevy::{ecs::system::Command, math::{Mat4, Quat, Vec3, Vec4}, prelude::{Commands, Entity, GlobalTransform, Query}, render2::render_resource::DynamicUniformVec};
use crevice::std140::AsStd140;

#[derive(Clone,Copy, Default, AsStd140)]
pub struct ExtractedSDFBrush {
    shape: u32,
    operation: u32,
    blending: f32,
    transform: Mat4,
    param1: Vec4,
    param2: Vec4
}

#[derive(Default)]
pub struct BrushUniform {
   pub brushes: DynamicUniformVec<ExtractedSDFBrush>,
}

#[derive(Default, Clone, AsStd140)]
pub struct BrushSettings {
    pub num_brushes: u32,
}

pub enum SDFShape {
    Sphere(f32),
    Box(f32, f32, f32),
}

pub enum SDFOperation {
    Union,
}

pub struct SDFBrush {
    pub order: u32,
    pub shape: SDFShape,
    pub operation: SDFOperation,
    pub blending: f32,
}

fn extract_sdf_brush(transform: &GlobalTransform, brush: &SDFBrush) -> ExtractedSDFBrush {
    let mut extracted = match brush.shape {
        SDFShape::Sphere(radius) => ExtractedSDFBrush { shape: 0, param1: Vec4::new(radius, 0., 0., 0.), ..Default::default()},
        SDFShape::Box(width, height, depth) => ExtractedSDFBrush{ shape: 1, param1: Vec4::new(width, height, depth, 0.), ..Default::default()},
    };
    extracted.transform = transform.compute_matrix();
    extracted.blending = brush.blending;
    extracted.operation = match brush.operation {
        SDFOperation::Union => 0,
    };
    extracted
}

pub fn extract_sdf_brushes(mut commands: Commands, brushes: Query<(Entity, &GlobalTransform, &SDFBrush)>) {
    for (entity, transform, brush) in brushes.iter() {
        commands.get_or_spawn(entity).insert(extract_sdf_brush(&transform, &brush));
    }
}