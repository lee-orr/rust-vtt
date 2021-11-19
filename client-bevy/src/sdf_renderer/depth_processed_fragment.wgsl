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
    let uv = in.uv;
    let result = textureSample(t_depth, s_depth, uv);
    let start_distance = (1. - result) * MAX_DISTANCE;
    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let ray = normalize(in.world_position - view.world_position);
    let hit = march(view.world_position + start_distance * ray, ray, in.pixel_size, MAX_DISTANCE,stack_pointer);
    //return vec4<f32>(hit.distance / MAX_DISTANCE, hit.jumps / f32(brush_settings.num_objects), f32(hit.iterations)/ f32(MAX_MARCHING_STEPS), 1.);
    if (hit.hit) {
        let norm = calculate_normal(hit.point, stack_pointer);
        let color = sceneColor(hit.point);
        out.color = vec4<f32>((color * clamp(norm.y, 0.2, 1.0)).x, hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS),1.0);
        out.depth = 1. - hit.distance / MAX_DISTANCE;
    } else {
        out.color = vec4<f32>(0.,hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.0);
        out.depth = 0.;
    }
    return out;
}