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
    var result : f32 = sceneSDF(position, voxel_size * 8., stack_pointer).x;
    result = clamp(result, -2., 6.) + 2.;
    result = result / 8.;
    textureStore(baked_map, vec3<i32>(global_invocation_id), vec4<f32>(result,0.,0.,0.));
}