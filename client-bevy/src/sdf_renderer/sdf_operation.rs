use bevy::{math::{Mat4, Quat, Vec3, Vec4}, render2::render_resource::DynamicUniformVec};
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