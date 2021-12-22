struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let ray = normalize(in.world_position - view.world_position);
    let hit = march(view.world_position + ray * view_extension.near, ray, in.pixel_size * 100., view_extension.far, stack_pointer);
    out.depth = hit.distance / view_extension.far;
    return out;
}