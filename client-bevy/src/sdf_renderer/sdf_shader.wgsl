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
[[group(1), binding(2)]]
var<storage, read> blocks: Blocks;

[[stage(vertex)]]
fn vs_main(
    vertex: Vertex,
    [[builtin(instance_index)]] instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let current_block = blocks.blocks[i32(instance_index)];
    var world_position: vec3<f32> = vertex.position * current_block.scale + current_block.position;

    out.clip_position = view.view_proj * vec4<f32>(world_position, 1.0);
    let ray = world_position - view.world_position;
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
    jumps: f32;
};

let MAX_MARCHING_STEPS = 100;
let MAX_DISTANCE = 100.0;
let NORM_EPSILON = 0.0005;
let MAX_BRUSH_DEPTH = 5;

let NORM_EPSILON_X = vec3<f32>(NORM_EPSILON, 0.0, 0.0);
let NORM_EPSILON_Y = vec3<f32>(0.0, NORM_EPSILON, 0.0);
let NORM_EPSILON_Z = vec3<f32>(0.0, 0.0, NORM_EPSILON);

let UNION_OP: i32 = 1;
let INTERSECTION_OP: i32 = 2;
let SUBTRACTION_OP: i32 = 3;
let TRANSFORM_WARP: i32 = 4;
let SPHERE_PRIM: i32 = 5;
let BOX_PRIM: i32 = 6;

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

fn transformSDF(point: vec3<f32>, matrix: mat4x4<f32>) -> vec3<f32> {
    return (matrix * vec4<f32>(point, 1.0)).xyz;
}

struct NodeStackItem {
    nodeid: i32;
    node: GpuSDFNode;
    child_a: f32;
    child_b: f32;
    processed_a: bool;
    processed_b: bool;
    process_bounds: bool;
    point: vec3<f32>;
    current_epsilon: f32;
};

fn setup_node(node: i32, current_node: i32, point: vec3<f32>, current_epsilon: f32, process_bounds: bool) -> NodeStackItem {
    var out : NodeStackItem;
    let id = node + current_node;
    out.node = brushes.brushes[id];
    out.nodeid = id;
    out.processed_a = false;
    out.processed_b = false;
    out.point = point;
    out.process_bounds = process_bounds;
    out.current_epsilon = current_epsilon;
    return out;
}

fn processNode(point: vec3<f32>, nodeid: i32, current_epsilon: f32, stack_ptr: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> vec2<f32> {
    var index : i32 = 0;
    var last_result : f32 = 99999999999.9;
    var num_jumps: f32 = 0.0;
    var stack = *stack_ptr;
    stack[0] = setup_node(nodeid, 0, point, current_epsilon, true);
    loop {
       num_jumps = f32(nodeid);
       if (index == -1 || index >= MAX_BRUSH_DEPTH) {
           break;
       }
        var enter_child: bool = false;
        var child_index = index + 1;
        var current_frame = stack[index];
        var node = current_frame.node;
        if (current_frame.process_bounds) {
            var d = distance(current_frame.point, node.center);
            if (d > node.radius + current_frame.current_epsilon) {
                last_result = d - node.radius + current_frame.current_epsilon/2.;
                index = index - 1;
                continue;
            } 
        }
        if (node.node_type == SPHERE_PRIM) {
            last_result = sphereSDF(current_frame.point, node.params[0].x);
        } elseif (node.node_type == BOX_PRIM) {
            last_result = boxSDF(current_frame.point, node.params[0].xyz);
        } elseif (node.node_type == TRANSFORM_WARP) {
            if (!current_frame.processed_a) {
                var new_point = transformSDF(current_frame.point, node.params);
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid, new_point, current_epsilon, false);
                enter_child = true;
                stack[index].processed_a = true;
            }
        } elseif (node.node_type == UNION_OP) {
            if (!current_frame.processed_a) {
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid,current_frame.point, max(node.params[0].x, current_epsilon), true);
                enter_child = true;
                stack[index].processed_a = true;
            } elseif (!current_frame.processed_b) {
                stack[index].child_a = last_result;
                stack[child_index] = setup_node(node.child_b, current_frame.nodeid,current_frame.point, max(node.params[0].x, current_epsilon), true);
                enter_child = true;
                stack[index].processed_b = true;
            } else {
                current_frame.child_b = last_result;
                if (node.params[0].x > 0.0) {
                    last_result = smoothUnionSDF(current_frame.child_a, current_frame.child_b, node.params[0].x);
                } else {
                    last_result = unionSDF(current_frame.child_a, current_frame.child_b);
                 }
            }
        } elseif (node.node_type == INTERSECTION_OP) {
            if (!current_frame.processed_a) {
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid,current_frame.point, max(node.params[0].x, current_epsilon), false);
                enter_child = true;
                stack[index].processed_a = true;
            } elseif (!current_frame.processed_b) {
                stack[index].child_a = last_result;
                stack[child_index] = setup_node(node.child_b, current_frame.nodeid,current_frame.point, max(node.params[0].x, current_epsilon), false);
                enter_child = true;
                stack[index].processed_b = true;
            } else {
                current_frame.child_b = last_result;
                if (node.params[0].x > 0.0) {
                    last_result = smoothIntersectionSDF(current_frame.child_a, current_frame.child_b, node.params[0].x);
                } else {
                    last_result = intersectionSDF(current_frame.child_a, current_frame.child_b);
                 }
            }
        }elseif (node.node_type == SUBTRACTION_OP) {
            if (!current_frame.processed_a) {
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid,current_frame.point, max(node.params[0].x, current_epsilon), false);
                enter_child = true;
                stack[index].processed_a = true;
            } elseif (!current_frame.processed_b) {
                stack[index].child_a = last_result;
                stack[child_index] = setup_node(node.child_b, current_frame.nodeid,current_frame.point, max(node.params[0].x, current_epsilon), true);
                enter_child = true;
                stack[index].processed_b = true;
            } else {
                current_frame.child_b = last_result;
                if (node.params[0].x > 0.0) {
                    last_result = smoothSubtractionSDF(current_frame.child_b, current_frame.child_a, node.params[0].x);
                } else {
                    last_result = subtractionSDF(current_frame.child_b, current_frame.child_a);
                 }
            }
        }
        if (enter_child) {
            index = child_index;
        } else {
            index = index - 1;
        }
   }
    return vec2<f32>(last_result, num_jumps);
}

fn sceneSDF(point: vec3<f32>, current_epsilon: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> vec2<f32> {
    var dist : f32 = 9999999999.9;
    var num_jumps : f32 = 0.0;
    let num_objects : i32 = i32(brush_settings.num_objects);
    let p : vec4<f32> = vec4<f32>(point.xyz, 1.0);
    for (var i : i32 = 0; i < num_objects; i = i + 1) {
        var result = processNode(point, i, current_epsilon, stack);
        var brush_dist : f32 = result.x;
        if (dist > brush_dist) {
            num_jumps = result.y;
        }
        dist = min(dist, brush_dist);
    }
    return vec2<f32>(dist, num_jumps);
}

fn sceneColor(point: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.7, 0.2, 0.2);
}

fn march(start: vec3<f32>, ray: vec3<f32>, pixel_size: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> MarchHit {
    let global_hit_epsilon: f32 = pixel_size;
    var last_epsilon: f32 = pixel_size;
    var depth : f32 = 0.5;
    var out : MarchHit;
    var jumps : f32 = 0.;
    for (var i : i32 = 0; i < MAX_MARCHING_STEPS; i = i + 1) {
        let offset = depth * ray;
        let point = start + offset;
        let distance_to_start = length(offset);
        let hit_epsilon = global_hit_epsilon * (view_extension.cone_scaler * distance_to_start);
        last_epsilon = hit_epsilon;
        let dist : vec2<f32> = sceneSDF(point, hit_epsilon * 10., stack);
        jumps = dist.y;
        if (dist.x < hit_epsilon) {
            out.distance = dist.x;
            out.point = point;
            out.hit = true;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.jumps = jumps;
            return out;
       } elseif ( distance_to_start > MAX_DISTANCE) {
            out.distance = depth;
            out.hit = false;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.jumps = jumps;
            return out;
        }
        
        depth = depth + dist.x;
    }
    out.final_epsilon = last_epsilon;
    out.distance = depth;
    out.hit = false;
    out.iterations = MAX_MARCHING_STEPS;
    out.jumps = jumps;
    return out;
}

fn calculate_normal(point: vec3<f32>, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>)-> vec3<f32> {
    var normal = vec3<f32>(
        sceneSDF(point + NORM_EPSILON_X, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_X, NORM_EPSILON, stack).x,
        sceneSDF(point + NORM_EPSILON_Y, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_Y, NORM_EPSILON, stack).x,
        sceneSDF(point + NORM_EPSILON_Z, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_Z, NORM_EPSILON, stack).x,
    );
    return normalize(normal);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.5, 0.2, 0.3, 1.);
    // var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    // let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    // let hit = march(view.world_position, in.ray_direction, in.pixel_size, stack_pointer);
    // //return vec4<f32>(hit.distance / MAX_DISTANCE, hit.jumps / f32(brush_settings.num_objects), f32(hit.iterations)/ f32(MAX_MARCHING_STEPS), 1.);
    // if (hit.hit) {
    //     let norm = calculate_normal(hit.point, stack_pointer);
    //     let color = sceneColor(hit.point);
    //     return vec4<f32>((color * clamp(norm.y, 0.2, 1.0)).x, hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS),1.0);
    // } else {
    //     return vec4<f32>(0.,hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.0);
    // }
}