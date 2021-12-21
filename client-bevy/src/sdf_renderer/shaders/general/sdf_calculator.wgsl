
let NORM_EPSILON = 0.0005;
let MAX_BRUSH_DEPTH = 10;

let NORM_EPSILON_X = vec3<f32>(NORM_EPSILON, 0.0, 0.0);
let NORM_EPSILON_Y = vec3<f32>(0.0, NORM_EPSILON, 0.0);
let NORM_EPSILON_Z = vec3<f32>(0.0, 0.0, NORM_EPSILON);

let UNION_OP: i32 = 1;
let INTERSECTION_OP: i32 = 2;
let SUBTRACTION_OP: i32 = 3;
let PAINT_OP: i32 = 3;
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
    blend: f32;
};

fn setup_node(node: i32, current_node: i32, point: vec3<f32>, current_epsilon: f32, process_bounds: bool, blend: f32) -> NodeStackItem {
    var out : NodeStackItem;
    let id = node + current_node;
    out.node = brushes.brushes[id];
    out.nodeid = id;
    out.processed_a = false;
    out.processed_b = false;
    out.point = point;
    out.process_bounds = process_bounds;
    out.current_epsilon = current_epsilon;
    out.blend = blend;
    return out;
}

fn bounding_sphere_intersection(origin: vec3<f32>, ray: vec3<f32>, radius: f32, blend: f32) -> f32 {
    let radius = radius + blend;
    if (length(ray) == 0.) {
        return sphereSDF(origin, radius);
    } else {
        let a = dot(ray, ray);
        let b = 2. * dot(origin, ray);
        let c = dot(origin, origin,) - radius * radius;
        let d = b * b - 4. * a * c;
        if (d < 0.) {
            return 999999999.;
        }
        let num = -b - sqrt(d);
        if (num > 0.) {
            return num / (2. * a);
        }

        let num2 = -b + sqrt(d);
        if (num2 > 0.) {
            return num2 / (2. * a);
        }
        return 9999999.;
    }
}

fn processNode(point: vec3<f32>, nodeid: i32, current_epsilon: f32, ray: vec3<f32>, stack_ptr: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> f32 {
    var index : i32 = 0;
    var last_result : f32 = 99999999999.9;
    var num_jumps: f32 = 0.0;
    var stack = *stack_ptr;
    stack[0] = setup_node(nodeid, 0, point, current_epsilon, true, 0.);
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
            var d = bounding_sphere_intersection(current_frame.point - node.center, ray, node.radius, current_frame.blend);
            // if (index > 1) {
            //     last_result = d;
            //     index = index - 1;
            //     continue;
            // }
            if (d > current_frame.current_epsilon * 2.) {
                last_result = d;
                index = index - 1;
                continue;
            }
        }
        if (node.node_type == SPHERE_PRIM) {
            last_result = sphereSDF(current_frame.point, node.params[0].x);
        } else if (node.node_type == BOX_PRIM) {
            last_result = boxSDF(current_frame.point, node.params[0].xyz);
        } else if (node.node_type == TRANSFORM_WARP) {
            if (!current_frame.processed_a) {
                var new_point = transformSDF(current_frame.point, node.params);
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid, new_point, current_frame.current_epsilon, false, current_frame.blend);
                enter_child = true;
                stack[index].processed_a = true;
            }
        } else if (node.node_type == UNION_OP) {
            if (!current_frame.processed_a) {
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid,current_frame.point,current_frame.current_epsilon, true, current_frame.blend + node.params[0].x);
                enter_child = true;
                stack[index].processed_a = true;
            } else if (!current_frame.processed_b) {
                stack[index].child_a = last_result;
                stack[child_index] = setup_node(node.child_b, current_frame.nodeid,current_frame.point,current_frame.current_epsilon, true, current_frame.blend + node.params[0].x);
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
        } else if (node.node_type == INTERSECTION_OP) {
            if (!current_frame.processed_a) {
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid,current_frame.point,current_frame.current_epsilon, false, current_frame.blend + node.params[0].x);
                enter_child = true;
                stack[index].processed_a = true;
            } else if (!current_frame.processed_b) {
                stack[index].child_a = last_result;
                stack[child_index] = setup_node(node.child_b, current_frame.nodeid,current_frame.point,current_frame.current_epsilon, false, current_frame.blend + node.params[0].x);
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
        }else if (node.node_type == SUBTRACTION_OP) {
            if (!current_frame.processed_a) {
                stack[child_index] = setup_node(node.child_a, current_frame.nodeid,current_frame.point,current_frame.current_epsilon, false, current_frame.blend + node.params[0].x);
                enter_child = true;
                stack[index].processed_a = true;
            } else if (!current_frame.processed_b) {
                stack[index].child_a = last_result;
                stack[child_index] = setup_node(node.child_b, current_frame.nodeid,current_frame.point,current_frame.current_epsilon, true, current_frame.blend + node.params[0].x);
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
    return last_result;
}

fn zoneSceneSDF(point: vec3<f32>, current_epsilon: f32, ray: vec3<f32>, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> f32 {
    let zone_size :vec3<f32> = num_zones.zone_size;
    let relative_pos = point - num_zones.zone_origin;
    let zone_id = relative_pos / zone_size;
    let zones_per_dimension = f32(num_zones.zones_per_dimension);
    if (zone_id.x >= zones_per_dimension || zone_id.y >= zones_per_dimension || zone_id.z >= zones_per_dimension || zone_id.x < 0. || zone_id.y < 0. || zone_id.z < 0.) {
        let adjusted_point = point - num_zones.world_center;
        return boxSDF(adjusted_point, num_zones.world_bounds) + current_epsilon;
    }
    let zone_index = i32(floor(zone_id.x)) * num_zones.zones_per_dimension * num_zones.zones_per_dimension
        + i32(floor(zone_id.y)) * num_zones.zones_per_dimension + i32(floor(zone_id.z));
    let zone = zones.zones[zone_index]; 
    
    let final_object : i32 = zone.final_object;
    let first_object : i32 = zone.first_object;
    var dist : f32 = num_zones.zone_radius;
    if (length(ray) > 0.) {
        var t1 : f32 = (zone.min.x - point.x) / ray.x;
        var t2 : f32 = (zone.max.x - point.x) / ray.x;

        var tmin : f32 = min(t1, t2);
        var tmax : f32 = max(t1, t2);

        t1 = (zone.min.y - point.y) / ray.y;
        t2 = (zone.max.y - point.y) / ray.y;

        tmin = max(tmin, min(t1, t2));
        tmax = min(tmax, max(t1, t2));

        t1 = (zone.min.z- point.z) / ray.z;
        t2 = (zone.max.z - point.z) / ray.z;

        tmin = max(tmin, min(t1, t2));
        tmax = min(tmax, max(t1, t2));
        
        dist = min(max(tmax, current_epsilon) + current_epsilon, dist);
    } else {
        let adjusted_point = point - zone.center;
        dist = min(max(current_epsilon * 20., -boxSDF(adjusted_point, num_zones.zone_half_size)) + current_epsilon, dist);
    }
    if (first_object == final_object) {
        return dist;
    }
    for (var i : i32 = first_object; i < final_object; i = i + 1) {
        let object_id = zone_objects.zone_objects[i];
        var brush_dist = processNode(point, object_id, num_zones.zone_radius, ray, stack);
        dist = min(dist, brush_dist);
    }
    return dist;
}

fn objectSceneSDF(point: vec3<f32>, current_epsilon: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> f32 {
    var dist : f32 = 99999.;
    for (var i : i32 = 0; i < num_objects.num_objects; i = i + 1) {
        var brush_dist = processNode(point, i, current_epsilon, vec3<f32>(0., 0., 0.), stack);
        dist = min(dist, brush_dist);
    }
    return dist;
}

fn sceneSDF(point: vec3<f32>, current_epsilon: f32, ray: vec3<f32>, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> f32 {
    return zoneSceneSDF(point, current_epsilon, ray, stack);
}

fn sceneColor(point: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.7, 0.2, 0.2);
}

fn calculate_normal(point: vec3<f32>, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>)-> vec3<f32> {
    let ray = vec3<f32>(0., 0., 0.);
    var normal = vec3<f32>(
        sceneSDF(point + NORM_EPSILON_X, NORM_EPSILON, ray, stack) - sceneSDF(point - NORM_EPSILON_X, NORM_EPSILON, ray,stack),
        sceneSDF(point + NORM_EPSILON_Y, NORM_EPSILON, ray,stack) - sceneSDF(point - NORM_EPSILON_Y, NORM_EPSILON, ray,stack),
        sceneSDF(point + NORM_EPSILON_Z, NORM_EPSILON, ray,stack) - sceneSDF(point - NORM_EPSILON_Z, NORM_EPSILON, ray,stack),
    );
    return normalize(normal);
}