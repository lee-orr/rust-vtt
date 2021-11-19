struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
  [[location(0)]] second_hit: f32;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let hit = march(view.world_position, normalize(in.world_position - view.world_position), in.pixel_size * 9., MAX_DISTANCE,stack_pointer);
    if (hit.hit && hit.distance < MAX_DISTANCE) {
        out.depth = 1. - hit.distance / MAX_DISTANCE;
    } else {
        out.depth = 0.;
    }
    out.second_hit = 0.;
    return out;
}