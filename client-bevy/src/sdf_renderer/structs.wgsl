// Vertex shader
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};

[[block]]
struct ViewExtension {
    view_proj_inverted: mat4x4<f32>;
    proj_inverted: mat4x4<f32>;
    cone_scaler: f32;
    pixel_size: f32;
};

struct SDFBrush {
    shape: i32;
    operation: i32;
    blending: f32;
    transform: mat4x4<f32>;
    param1: vec4<f32>;
    param2: vec4<f32>;
};

struct GpuSDFNode {
    node_type: i32;
    child_a: i32;
    child_b: i32;
    params: mat4x4<f32>;
    radius: f32;
    center: vec3<f32>;
};

struct GpuSDFBlock {
    scale: f32;
    position: vec3<f32>;
};

[[block]]
struct Brushes {
    brushes: array<GpuSDFNode>;
};

[[block]]
struct Blocks {
    blocks: array<GpuSDFBlock>;
};

[[block]]
struct BrushSettings {
    num_objects: i32;
};

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec3<f32>;
    [[location(1)]] pixel_size: f32;
    [[location(2)]] max_distance: f32;
    [[location(3)]] uv: vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;
[[group(1), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(1), binding(1)]]
var<uniform> brush_settings: BrushSettings;
[[group(1), binding(2)]]
var<storage, read> blocks: Blocks;
[[group(2), binding(0)]]
var t_depth: texture_depth_2d;
[[group(2), binding(1)]]
var s_depth: sampler;
[[group(2), binding(2)]]
var t_hits: texture_depth_2d;
[[group(2), binding(3)]]
var s_hits: sampler;
