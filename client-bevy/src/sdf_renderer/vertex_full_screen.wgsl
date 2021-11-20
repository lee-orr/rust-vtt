[[stage(vertex)]]
fn vs_main(
    vertex: Vertex,
) -> VertexOutput {
    var out: VertexOutput;
    let x = vertex.position.x;
    let y = vertex.position.y;
    out.clip_position = vec4<f32>(x, y, 0.1, 1.0);
    out.uv = vertex.uv;
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