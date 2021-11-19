struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    if (in.clip_position.x > 0.) {
        out.depth = 1.;
    } else {
        out.depth = 0.1;
    }
    // var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    // let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    // let hit = march(view.world_position, normalize(in.world_position - view.world_position), in.pixel_size, MAX_DISTANCE,stack_pointer);
    // if (hit.hit) {
    //     out.depth = 1. - hit.distance / MAX_DISTANCE;
    // } else {
    //     out.depth = 0.;
    // }
    return out;
}