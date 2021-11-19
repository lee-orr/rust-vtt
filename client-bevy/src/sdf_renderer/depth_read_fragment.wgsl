struct FragmentOut {
  [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    let uv = in.uv;
    let result = textureSample(t_depth, s_depth, uv);
    out.color = vec4<f32>(result, result, result, 1.0);
    return out;
}