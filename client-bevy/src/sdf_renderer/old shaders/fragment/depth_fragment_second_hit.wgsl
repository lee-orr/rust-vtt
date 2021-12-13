struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
  [[location(0)]] second_hit: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let hit = march(view.world_position, normalize(in.world_position - view.world_position), in.pixel_size * 8., MAX_DISTANCE,stack_pointer);
    if (hit.hit && hit.distance < MAX_DISTANCE) {
        out.depth = hit.distance;
        out.second_hit.y = out.depth;
    } else {
        out.depth = MAX_DISTANCE;
        out.second_hit.y = MAX_DISTANCE;
    }
    if (hit.hit_2 && hit.distance_2 < MAX_DISTANCE) {
        out.second_hit.x = hit.distance_2;
    } else {
        out.second_hit.x = MAX_DISTANCE;
    }
    out.second_hit.z = hit.final_epsilon;
    out.second_hit.w = hit.left_first_object;
    return out;
}