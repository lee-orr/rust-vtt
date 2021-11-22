[[stage(compute), workgroup_size(8, 8, 8)]]
fn cmp_main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let voxels_per_layer : vec3<f32> = bake_settings.layer_size;
    let num_layers = f32(bake_settings.num_layers);
    let current_layer = global_invocation_id.z / u32(voxels_per_layer.z);
    let voxel_in_layer = vec3<f32>((f32(global_invocation_id.x) + 0.5) / voxels_per_layer.x, (0.5 + f32(global_invocation_id.y)) / voxels_per_layer.y, (0.5  + f32(global_invocation_id.z - current_layer * u32(bake_settings.layer_size.z))) / voxels_per_layer.z);
    let layer_power = num_layers - f32(current_layer) - 1.;
    let layer_size = bake_settings.max_size / pow(f32(bake_settings.layer_multiplier), layer_power);
    let voxel_size = max_component(layer_size/voxels_per_layer);
    var position : vec3<f32> = layer_size * (voxel_in_layer - 0.5) - baker_origins.origin;
    let epsilon = voxel_size / 4.;
    let pos_a = position + vec3<f32>(epsilon, 0., 0.);
    let pos_b = position + vec3<f32>( 0.,epsilon, 0.);
    let pos_c = position + vec3<f32>(0., 0., epsilon);
    let pos_d = position + vec3<f32>( 0.,0., 0.);
    let r_a = sceneSDF(pos_a, voxel_size, stack_pointer);
    let r_b = sceneSDF(pos_b, voxel_size, stack_pointer);
    let r_c = sceneSDF(pos_c, voxel_size, stack_pointer);
    let r_d = sceneSDF(pos_d, voxel_size, stack_pointer);
    var result : f32 = min(min(r_a.x, r_b.x),min(r_c.x, r_d.x));
    result = clamp(result, -2., 6.) + 2.;
    result = result / 8.;
    let normal = 0.5 + 0.5 * normalize(vec3<f32>(r_a.x - r_d.x, r_b.x - r_d.x, r_c.x - r_d.x));
    textureStore(baked_map, vec3<i32>(global_invocation_id), vec4<f32>(result, normal.x, normal.y, normal.z));
}