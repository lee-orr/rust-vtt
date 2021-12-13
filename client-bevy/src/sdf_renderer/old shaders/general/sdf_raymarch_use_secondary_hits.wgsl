struct MarchHit {
    distance: f32;
    point: vec3<f32>;
    normal: vec3<f32>;
    hit: bool;
    iterations: i32;
    final_epsilon: f32;
    jumps: f32;
};

let MAX_MARCHING_STEPS = 100;
let MAX_DISTANCE = 100.;

fn march(start: vec3<f32>, ray: vec3<f32>, start_distance: f32, pixel_size: f32, max_dist: f32, max_steps: i32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> MarchHit {
    let global_hit_epsilon: f32 = pixel_size;
    var last_epsilon: f32 = pixel_size;
    var depth : f32 = start_distance;
    var normal : vec3<f32>;
    var last_distance : f32 = MAX_DISTANCE;
    var max_depth = min(max_dist, MAX_DISTANCE);
    var out : MarchHit;
    var jumps : f32 = 0.;
    for (var i : i32 = 0; i < max_steps; i = i + 1) {
        let offset = depth * ray;
        let point = start + offset;
        let distance_to_start = length(offset);
        let hit_epsilon = global_hit_epsilon * (view_extension.cone_scaler * distance_to_start);
        last_epsilon = hit_epsilon;
        let dist : vec4<f32> = sceneSDF(point, hit_epsilon, stack);
        normal = dist.yzw;
        if (dist.x < hit_epsilon) {
            out.distance = depth + dist.x;
            out.point = point;
            out.hit = true;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.jumps = jumps;
            out.normal = normal;
            return out;
       } elseif ( distance_to_start > max_depth) {
            out.distance = depth;
            out.hit = false;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.jumps = jumps;
            out.normal = normal;
            return out;
        }
        last_distance = dist.x;        
        depth = depth + dist.x;
    }
    out.final_epsilon = last_epsilon;
    out.distance = depth;
    if (last_distance < last_epsilon) {
        out.hit = true;
    } else {
        out.hit = false;
    }
    out.iterations = max_steps;
    out.jumps = jumps;
    out.normal = normal;
    return out;
}