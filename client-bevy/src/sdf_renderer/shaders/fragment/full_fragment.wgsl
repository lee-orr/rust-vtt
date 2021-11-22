struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
  [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;
    // let dist = in.max_distance/MAX_DISTANCE;
    // let height = clamp(in.world_position.y / 3. + 0.5, 0., 1.);
    // out.color = vec4<f32>(height * dist, 0.,height * (1. - dist), 1.);
   // out.depth = 1.;
    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let hit = march(view.world_position, normalize(in.world_position - view.world_position), in.pixel_size, MAX_DISTANCE,stack_pointer);
    //return vec4<f32>(hit.distance / MAX_DISTANCE, hit.jumps / f32(brush_settings.num_objects), f32(hit.iterations)/ f32(MAX_MARCHING_STEPS), 1.);
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