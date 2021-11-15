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

[[block]]
struct Brushes {
    brushes: array<SDFBrush>;
};

[[block]]
struct BrushSettings {
    num_brushes: i32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] ray_direction: vec3<f32>;
    [[location(1)]] pixel_size: f32;
};

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;
[[group(1), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(1), binding(1)]]
var<uniform> brush_settings: BrushSettings;

[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 5.;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 5.;
    out.clip_position = vec4<f32>(x, y, 0.1, 1.0);
    let view_space_position = view_extension.view_proj_inverted * out.clip_position;
    let ray = view_space_position.xyz - view.world_position;
    out.ray_direction = normalize(ray);
    let clip_space_center = vec4<f32>(0.,0.,0., 1.);
    let clip_space_one = vec4<f32>(view_extension.pixel_size, 0., 0., 1.);
    let view_space_center = view_extension.view_proj_inverted * clip_space_center;
    let view_space_one = view_extension.view_proj_inverted * clip_space_one;
    let pixel_size = length(view_space_one - view_space_center);
    out.pixel_size = pixel_size;
    return out;
}


// Fragment shader
struct MarchHit {
    distance: f32;
    point: vec3<f32>;
    hit: bool;
    iterations: i32;
    final_epsilon: f32;
};

let MAX_MARCHING_STEPS = 100;
let MAX_DISTANCE = 100.0;
let NORM_EPSILON = 0.0005;

let NORM_EPSILON_X = vec3<f32>(NORM_EPSILON, 0.0, 0.0);
let NORM_EPSILON_Y = vec3<f32>(0.0, NORM_EPSILON, 0.0);
let NORM_EPSILON_Z = vec3<f32>(0.0, 0.0, NORM_EPSILON);

let SPHERE_CODE = 0;
let SQUARE_CODE = 1;

let UNION_CODE = 0;
let SUBTRACTION_CODE = 1;
let INTERSECTION_CODE = 2;

fn sphereSDF(point: vec3<f32>, radius: f32) -> f32 {
    return length(point) - radius;
}

fn boxSDF(point: vec3<f32>, bounds: vec3<f32>) -> f32 {
    let quadrant = abs(point) - bounds;
    return length(max(quadrant,vec3<f32>(0.0, 0.0, 0.0))) + min(max(quadrant.x,max(quadrant.y,quadrant.z)),0.0);
}

fn unionSDF(a: f32, b: f32) -> f32 {
    return min(a, b);
}

fn smoothUnionSDF(a: f32, b: f32, smoothness: f32) -> f32 {
    let h = max(smoothness - abs(a - b), 0.0)/smoothness;
    return min(a,b) - h * h* smoothness * (1.0/4.0);
}

fn subtractionSDF(a: f32, b: f32) -> f32 {
    return max(-a, b);
}

fn smoothSubtractionSDF(a: f32, b: f32, smoothness: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (a + b)/smoothness, 0.0, 1.0);

    return mix (b, -a, h) + smoothness * h * (1.0 - h);
}

fn intersectionSDF(a: f32, b: f32) -> f32 {
    return max(a, b);
}

fn smoothIntersectionSDF(a: f32, b: f32, smoothness: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (b-a)/smoothness, 0.0, 1.0);

    return mix (b, a, h) + smoothness * h * (1.0 - h);
}

fn sceneSDF(point: vec3<f32>) -> f32 {
    var dist : f32 = 9999999999.9;
    let num_brushes : i32 = i32(brush_settings.num_brushes);
    let p : vec4<f32> = vec4<f32>(point.xyz, 1.0);
    for (var i : i32 = 0; i < num_brushes; i = i + 1) {
        let brush = brushes.brushes[i];
        var brush_dist : f32 = 9999999999.9;
        let transform : mat4x4<f32> =  brush.transform;
        let transformed_point = (transform * p).xyz;
         if (brush.shape == SPHERE_CODE) {
            brush_dist = sphereSDF(transformed_point, brush.param1.x);
        } elseif (brush.shape == SQUARE_CODE) {
            brush_dist = boxSDF(transformed_point, brush.param1.xyz);
        }

        if (brush.operation == UNION_CODE) {
            if (brush.blending > 0.0) {
                dist = smoothUnionSDF(dist, brush_dist, brush.blending);
            } else {
                dist = unionSDF(dist, brush_dist);
            }
        } elseif (brush.operation == SUBTRACTION_CODE) {
            if (brush.blending > 0.0) {
                dist = smoothSubtractionSDF(brush_dist, dist, brush.blending);
            } else {
                dist = subtractionSDF(brush_dist, dist);
            }
        } elseif (brush.operation == INTERSECTION_CODE) {
            if (brush.blending > 0.0) {
                dist = smoothIntersectionSDF(brush_dist, dist, brush.blending);
            } else {
                dist = intersectionSDF(brush_dist, dist);
            }
        }
    }
    return dist;
}

fn sceneColor(point: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.7, 0.2, 0.2);
}

fn march(start: vec3<f32>, ray: vec3<f32>, pixel_size: f32) -> MarchHit {
    let global_hit_epsilon: f32 = pixel_size;
    var last_epsilon: f32 = pixel_size;
    var depth : f32 = 0.5;
    var out : MarchHit;
    for (var i : i32 = 0; i < MAX_MARCHING_STEPS; i = i + 1) {
        let offset = depth * ray;
        let point = start + offset;
        let distance_to_start = length(offset);
        let hit_epsilon = global_hit_epsilon * (view_extension.cone_scaler * distance_to_start);
        last_epsilon = hit_epsilon;
        let dist = sceneSDF(point);
        if (dist < hit_epsilon) {
            out.distance = dist;
            out.point = point;
            out.hit = true;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            return out;
        } elseif ( distance_to_start > MAX_DISTANCE) {
            out.distance = depth;
            out.hit = false;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            return out;
        }
        
        depth = depth + dist;
    }
    out.final_epsilon = last_epsilon;
    out.distance = depth;
    out.hit = false;
    out.iterations = MAX_MARCHING_STEPS;
    return out;
}

fn calculate_normal(point: vec3<f32>)-> vec3<f32> {
    var normal = vec3<f32>(
        sceneSDF(point + NORM_EPSILON_X) - sceneSDF(point - NORM_EPSILON_X),
        sceneSDF(point + NORM_EPSILON_Y) - sceneSDF(point - NORM_EPSILON_Y),
        sceneSDF(point + NORM_EPSILON_Z) - sceneSDF(point - NORM_EPSILON_Z),
    );
    return normalize(normal);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let hit = march(view.world_position, in.ray_direction, in.pixel_size);
    if (hit.hit) {
        let norm = calculate_normal(hit.point);
        let color = sceneColor(hit.point);
        return vec4<f32>((color * clamp(norm.y, 0.2, 1.0)).x, hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS),1.0);
    } else {
        return vec4<f32>(0.,hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.0);
    }
}