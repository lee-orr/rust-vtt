[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 5.;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 5.;
    out.clip_position = vec4<f32>(x, y, 0.1, 1.0);
    let view_space_position = view_extension.view_proj_inverted * out.clip_position;
    let ray = view_space_position.xyz - view.world_position;
    out.world_position = view_space_position.xyz;
    let clip_space_center = vec4<f32>(0.,0.,0., 1.);
    let clip_space_one = vec4<f32>(view_extension.pixel_size, 0., 0., 1.);
    let view_space_center = view_extension.view_proj_inverted * clip_space_center;
    let view_space_one = view_extension.view_proj_inverted * clip_space_one;
    let pixel_size = length(view_space_one - view_space_center);
    out.pixel_size = pixel_size;
    out.max_distance = 999999999999.;
    return out;
}