struct MarchHit {
    distance: f32;
    point: vec3<f32>;
    hit: bool;
    iterations: i32;
    final_epsilon: f32;
    jumps: f32;
};

let MAX_MARCHING_STEPS = 100;
let MAX_DISTANCE = 100.;
let MAX_SMALL_STEPS = 5;
let SMALL_STEP_THRESHOLD = 10.;

fn march(start: vec3<f32>, ray: vec3<f32>, pixel_size: f32, max_dist: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> MarchHit {
    let global_hit_epsilon: f32 = pixel_size;
    var last_epsilon: f32 = pixel_size;
    var depth : f32 = pixel_size;
    var max_depth = min(max_dist, view_extension.far);
    var out : MarchHit;
    var closest : f32 = view_extension.far;
    var num_small_steps : i32 = 0;
    for (var i : i32 = 0; i < MAX_MARCHING_STEPS; i = i + 1) {
        let offset = depth * ray;
        let point = start + offset;
        let distance_to_start = length(offset);
        let hit_epsilon = global_hit_epsilon * (view_extension.cone_scaler * distance_to_start);
        last_epsilon = hit_epsilon;
        let threshold = hit_epsilon * SMALL_STEP_THRESHOLD;
        var dist : f32 = sceneSDF(point, threshold, ray, stack);
        if (dist < threshold) {
            num_small_steps = num_small_steps + 1;
        } else {
            num_small_steps = 0;
        }
        if (num_small_steps > MAX_SMALL_STEPS) {
            dist = 0.;
        }
        closest = min(view_extension.far, dist);
        if (dist < hit_epsilon) {
            out.distance = dist;
            out.point = point;
            out.hit = true;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.jumps = closest;
            return out;
       } else if ( distance_to_start > max_depth) {
            out.distance = depth;
            out.hit = false;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.jumps = closest;
            return out;
        }
        
        depth = depth + max(dist, 2. * hit_epsilon);
    }
    out.final_epsilon = last_epsilon;
    out.distance = depth;
    out.hit = false;
    out.iterations = MAX_MARCHING_STEPS;
    out.jumps = closest;
    return out;
}