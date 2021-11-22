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
    let sample = textureSample(t_hits, s_hits, uv);
    let start_distance = sample.y;
    let second_hit_distance = sample.x;
    let last_epsilon = sample.z;
    let left_first_object = sample.w;

    if (start_distance >= MAX_DISTANCE) {
        out.color = vec4<f32>(0., 0., 0., 1.);
        out.depth = 0.;
        return out;
    }
    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let ray = normalize(in.world_position - view.world_position);
    var hit : MarchHit = march(view.world_position, ray, start_distance - last_epsilon * 10., in.pixel_size, left_first_object + last_epsilon * 10., MAX_MARCHING_STEPS, stack_pointer);
    // in.pixel_size * 8. * start_distance * view_extension.cone_scaler
    //return vec4<f32>(hit.distance / MAX_DISTANCE, hit.jumps / f32(brush_settings.num_objects), f32(hit.iterations)/ f32(MAX_MARCHING_STEPS), 1.);

    if (!hit.hit && second_hit_distance < MAX_DISTANCE) {
         hit = march(view.world_position, ray, second_hit_distance - last_epsilon * 10., in.pixel_size, MAX_DISTANCE, MAX_MARCHING_STEPS, stack_pointer);
    }

    if (hit.hit) {
        let norm = hit.normal;
        let color = sceneColor(hit.point);
        out.color = vec4<f32>((color * clamp(norm.y, 0.2, 1.0)).x, 0., f32(hit.iterations)/f32(MAX_MARCHING_STEPS),1.0);
        out.depth = 1. - hit.distance / MAX_DISTANCE;
    } else {
        out.color = vec4<f32>(0., hit.distance / MAX_DISTANCE, f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.);//vec4<f32>(0.,hit.final_epsilon / (view_extension.pixel_size * 100.), f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.0);
        out.depth = 0.;
    }
    return out;
}