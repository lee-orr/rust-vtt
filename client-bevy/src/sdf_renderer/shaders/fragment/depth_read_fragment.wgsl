struct FragmentOut {
  [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    let uv = in.uv;
    let result = textureSample(t_depth, s_depth, uv);
    let second_hit = textureSample(t_hits, s_hits, uv).xy;
    out.color = vec4<f32>(1. - 5. * second_hit.y / MAX_DISTANCE, 0., 1. - 5. * second_hit.x / MAX_DISTANCE, 1.0);
    return out;
}