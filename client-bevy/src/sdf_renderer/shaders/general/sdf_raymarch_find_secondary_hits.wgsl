struct MarchHit {
    distance: f32;
    distance_2: f32;
    left_first_object: f32;
    point: vec3<f32>;
    hit: bool;
    hit_2: bool;
    iterations: i32;
    final_epsilon: f32;
    jumps: f32;
};

let MAX_MARCHING_STEPS = 100;
let MAX_DISTANCE = 100.0;

fn march(start: vec3<f32>, ray: vec3<f32>, pixel_size: f32, max_dist: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> MarchHit {
    let global_hit_epsilon: f32 = pixel_size;
    var last_epsilon: f32 = pixel_size;
    var depth : f32 = pixel_size;
    var max_depth = min(max_dist, MAX_DISTANCE);
    var out : MarchHit;
    var jumps : f32 = 0.;
    var first_hit : f32 = depth;
    var left_first_object: f32 = depth;
    var second_hit : f32 = MAX_DISTANCE;
    var found_first_hit : bool = false;
    var left_first_hit : bool = false;
    var found_second_hit : bool = false;
    for (var i : i32 = 0; i < MAX_MARCHING_STEPS; i = i + 1) {
        let offset = depth * ray;
        let point = start + offset;
        let distance_to_start = length(offset);
        let hit_epsilon = global_hit_epsilon * (view_extension.cone_scaler * distance_to_start);
        last_epsilon = hit_epsilon;
        let dist : vec2<f32> = sceneSDF(point, hit_epsilon, stack);
        jumps = dist.y;
        if (dist.x < hit_epsilon) {
            depth = depth + hit_epsilon * 10.;
            if (!found_first_hit) {
                found_first_hit = true;
                first_hit = depth;
                left_first_object = depth + last_epsilon;
                second_hit = depth + last_epsilon;
            } elseif (!left_first_hit) {
                out.distance = first_hit;
                out.distance_2 = second_hit;
                out.hit = found_first_hit;
                out.hit_2 = found_second_hit;
                out.iterations = i;
                out.final_epsilon = last_epsilon;
                out.left_first_object = left_first_object;
                out.jumps = jumps;
                return out;
            } elseif (!found_second_hit) {
                found_second_hit = true;
                second_hit = depth;
            }
        } elseif (found_first_hit && !left_first_hit && dist.x > 0.) {
            left_first_hit = true;
            left_first_object = depth + dist.x + last_epsilon;
        }

        if (found_second_hit) {
            out.distance = first_hit;
            out.distance_2 = second_hit;
            out.point = point;
            out.hit = true;
            out.hit_2 = true;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.left_first_object = left_first_object;
            out.jumps = jumps;
            return out;
       } elseif ( distance_to_start > max_depth) {
            out.distance = first_hit;
            out.distance_2 = second_hit;
            out.hit = found_first_hit;
            out.hit_2 = found_second_hit;
            out.iterations = i;
            out.final_epsilon = last_epsilon;
            out.left_first_object = left_first_object;
            out.jumps = jumps;
            return out;
        }
        
        depth = depth + abs(dist.x);
    }
    out.final_epsilon = last_epsilon;
    out.distance = depth;
    out.hit = false;
    out.iterations = MAX_MARCHING_STEPS;
    out.left_first_object = left_first_object;
    out.jumps = jumps;
    return out;
}