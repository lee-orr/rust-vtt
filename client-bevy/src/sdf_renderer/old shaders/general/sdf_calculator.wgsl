
let NORM_EPSILON = 0.0005;
let MAX_BRUSH_DEPTH = 10;

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
        // if (current_frame.process_bounds) {
        //     // var d = length(point - node.center);
        //     // let radius_extension = current_frame.current_epsilon;
        //     // let threshold = node.radius + radius_extension;
        //     // if (d > threshold) {
        //     //     last_result = d - threshold + radius_extension / 2.;
        //     //     index = index - 1;
        //     //     continue;
        //     // }
        //     var d = point - node.center;
        //     let threshold = node.radius + current_frame.current_epsilon;
        //     last_result = sphereSDF(d, node.radius);
        //     index = index - 1;
        //     continue;
        // }
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
    var dist : f32 = num_zones.zone_radius;
    let zone_size :vec3<f32> = num_zones.zone_size;
    let relative_pos = point - num_zones.zone_origin;
    let zone_id = relative_pos / zone_size;
    let zones_per_dimension = f32(num_zones.zones_per_dimension);
    if (zone_id.x >= zones_per_dimension || zone_id.y >= zones_per_dimension || zone_id.z >= zones_per_dimension) {
        return vec2<f32>(dist, 0.);
    }
    let zone_index = i32(floor(zone_id.x)) * num_zones.zones_per_dimension * num_zones.zones_per_dimension
        + i32(floor(zone_id.y)) * num_zones.zones_per_dimension + i32(floor(zone_id.z));
    let zone = zones.zones[zone_index]; 
    
    let final_object : i32 = zone.final_object;
    let first_object : i32 = zone.first_object;
    for (var i : i32 = first_object; i < final_object; i = i + 1) {
        let object_id = zone_objects.zone_objects[i];
        var result = processNode(point, object_id, num_zones.zone_radius, stack);
        var brush_dist : f32 = result.x;
        dist = min(dist, brush_dist);
    } 
    return vec2<f32>(dist, 0.);
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