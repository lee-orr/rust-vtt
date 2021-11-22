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

[[block]]
struct SDFBakerSettings {
    max_size: vec3<f32>;
    layer_size: vec3<f32>;
    num_layers: u32;
    layer_multiplier: u32;
};

[[block]]
struct SDFBakedLayerOrigins {
    origin: vec3<f32>;
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

[[block]]
struct Brushes {
    brushes: array<GpuSDFNode>;
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


fn max_component(point: vec3<f32>) -> f32 {
    return max(point.x, max(point.y, point.z));
}
fn min_component(point: vec3<f32>) -> f32 {
    return min(point.x, max(point.y, point.z));
}