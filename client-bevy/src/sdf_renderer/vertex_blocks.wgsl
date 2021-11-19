[[stage(vertex)]]
fn vs_main(
    vertex: Vertex,
    [[builtin(instance_index)]] instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let current_block = blocks.blocks[i32(instance_index)];
    var world_position: vec3<f32> = vertex.position * current_block.scale + current_block.position;

    out.clip_position = view.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    let clip_space_center = vec4<f32>(0.,0.,0., 1.);
    let clip_space_one = vec4<f32>(view_extension.pixel_size, 0., 0., 1.);
    let view_space_center = view_extension.view_proj_inverted * clip_space_center;
    let view_space_one = view_extension.view_proj_inverted * clip_space_one;
    let pixel_size = length(view_space_one - view_space_center);
    out.pixel_size = pixel_size;
    let ray = world_position - view.world_position;
    out.max_distance = length(ray) + current_block.scale;
    return out;
}