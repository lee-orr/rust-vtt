// Vertex shader
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};


[[block]]
struct ViewExtension {
    view_proj_inverted: mat4x4<f32>;
    proj_inverted: mat4x4<f32>;
    cone_scaler: f32;
    pixel_size: f32;
    near: f32;
    far: f32;
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

struct GpuSDFNode {
    node_type: i32;
    child_a: i32;
    child_b: i32;
    params: mat4x4<f32>;
    radius: f32;
    center: vec3<f32>;
    color: vec3<f32>;
};

struct Zone {
    min: vec3<f32>;
    max: vec3<f32>;
    center: vec3<f32>;
    first_object: i32;
    final_object: i32;
};

[[block]]
struct Brushes {
    brushes: array<GpuSDFNode>;
};

[[block]]
struct SDFObjectCount {
    num_objects: i32;
};

[[block]]
struct Zones {
    zones: array<Zone>;
};

[[block]]
struct ZoneObjects {
    zone_objects: array<i32>;
};

[[block]]
struct ZoneSettings {
    num_zones: i32;
    zone_radius: f32;
    zone_size: vec3<f32>;
    zone_half_size: vec3<f32>;
    zone_origin: vec3<f32>;
    zones_per_dimension: i32;
    world_center: vec3<f32>;
    world_bounds: vec3<f32>;
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

struct BakedNode {
    node_type: i32; //0 - empty, 1 - full, 2 - contains block with children, 3 - contains block no children
    content: vec4<f32>; // either: the color & opacity (if it doesn't contain a surface) or (block_uvw.xyz, child_index) if it does
    parent: i32; // if (-1) it is a root, if < -1 it has been cleared
};

[[block]]
struct Nodes {
    last_written: atomic<i32>;
    nodes: array<BakedNode>;
};