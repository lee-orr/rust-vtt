struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
  [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let ray = normalize(in.world_position - view.world_position);
    let hit = march(view.world_position + ray * view_extension.near, ray, in.pixel_size, MAX_DISTANCE,stack_pointer);
    if (hit.hit) {
        let norm = calculate_normal(hit.point, stack_pointer);
        let color = sceneColor(hit.point);
        out.color = vec4<f32>((color * clamp(norm.y, 0.2, 1.0)).x, hit.jumps / MAX_DISTANCE, f32(hit.iterations)/f32(MAX_MARCHING_STEPS),1.0);
        out.depth = 1. - hit.distance / MAX_DISTANCE;
    } else {
        out.color = vec4<f32>(0., hit.jumps / MAX_DISTANCE, f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.0);
        out.depth = 1.;
    }
    return out;
}