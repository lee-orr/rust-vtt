struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
  [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;
    
    let uv = in.uv;
    let depth_sample = textureSample(t_depth, s_depth, uv);
    let start_distance = 0.9 * depth_sample * view_extension.far;

    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let ray = normalize(in.world_position - view.world_position);
    let hit = march(view.world_position + ray * start_distance, ray, in.pixel_size, view_extension.far,stack_pointer);
    if (hit.hit) {
        let norm = calculate_normal(hit.point, stack_pointer);
        let color = sceneColor(hit.point);
        out.color = vec4<f32>((color * clamp(norm.y, 0.2, 1.0)).x, hit.distance / view_extension.far, f32(hit.iterations)/f32(MAX_MARCHING_STEPS),1.0);
    } else {
        out.color = vec4<f32>(0., hit.distance / view_extension.far, f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.);
    }
        out.depth = 1. - hit.distance / view_extension.far;
    return out;
}