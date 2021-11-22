
let NORM_EPSILON = 0.0005;
let MAX_BRUSH_DEPTH = 1;

struct NodeStackItem {};

let NORM_EPSILON_X = vec3<f32>(NORM_EPSILON, 0.0, 0.0);
let NORM_EPSILON_Y = vec3<f32>(0.0, NORM_EPSILON, 0.0);
let NORM_EPSILON_Z = vec3<f32>(0.0, 0.0, NORM_EPSILON);

fn sceneSDF(point: vec3<f32>, current_epsilon: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> vec4<f32> {
    var relative_position : vec3<f32> = -point + baker_origins.origin;
    var level_size : vec3<f32> = baker_settings.max_size;
    var current_level : i32 = i32(baker_settings.num_layers) - 1;
    let voxels_per_layer : vec3<f32> = baker_settings.layer_size;
    var level_voxel_size: f32 = max_component(level_size / voxels_per_layer);
    let layer_multiplier : f32 = f32(baker_settings.layer_multiplier);
    let relative_position_dist = abs(relative_position) * 2.;

    if (any(relative_position_dist > level_size)) { return vec4<f32>(10000., 0., 0., 0.); }

    loop {
        if (current_level == 0 || level_voxel_size <= current_epsilon) { break; }
        let next_level_size = level_size / layer_multiplier;
        let next_level_voxel_size = max_component(level_size / voxels_per_layer);
        if (any(relative_position_dist + (next_level_voxel_size/2.) > next_level_size)) { break; }
        current_level = current_level - 1;
        level_size = next_level_size;
        level_voxel_size =next_level_voxel_size;
    }

    var uvw : vec3<f32> = relative_position/level_size + 0.5;
    if (max_component(uvw) >= 1. || min_component(uvw) <= 0.) {
        return vec4<f32>(10000., 0., 0., 0.);
    }
    let num_layers = f32(baker_settings.num_layers);
    uvw.z = (uvw.z + f32(current_level)) / num_layers;
    let sample = textureSampleLevel(t_baked, s_baked, uvw, 0.);
    let value = sample.x * 8. - 2.;
    let normal = -normalize(sample.yzw - 0.5);
    return vec4<f32>(value, normal);
}

fn sceneColor(point: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.7, 0.2, 0.2);
}

fn calculate_normal(point: vec3<f32>, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>)-> vec3<f32> {
    var normal = vec3<f32>(
        sceneSDF(point + NORM_EPSILON_X, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_X, NORM_EPSILON, stack).x,
        sceneSDF(point + NORM_EPSILON_Y, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_Y, NORM_EPSILON, stack).x,
        sceneSDF(point + NORM_EPSILON_Z, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_Z, NORM_EPSILON, stack).x,
    );
    return normalize(normal);
}