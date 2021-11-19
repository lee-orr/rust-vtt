struct FragmentOut {
  [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    let clip = (in.clip_position.xy)*view_extension.pixel_size;
    let result = textureSample(t_depth, s_depth, clip);
    out.color = vec4<f32>(clip.x, clip.y, result, 1.0);
    return out;
}