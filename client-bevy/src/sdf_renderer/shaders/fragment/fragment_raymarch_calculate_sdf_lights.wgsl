struct FragmentOut {
  [[builtin(frag_depth)]] depth: f32;
  [[location(0)]] color: vec4<f32>;
};

fn calculate_light(point: vec3<f32>, view_ray: vec3<f32>, norm: vec3<f32>, color: vec3<f32>, epsilon: f32, light: Light, stack_ptr: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> vec3<f32> {
    if (light.params[0].w < 0.5) {
        let light_pos = light.params[0].xyz;
        let distance = length(light_pos - point);
        if (distance < light.params[2].x) {
            let direction = normalize(light_pos - point);
            let hit = march(point + epsilon * 2. * direction, direction, epsilon, distance, stack_ptr);
            if (!hit.hit) {
                let distance = distance * distance;
                let normal_to_light = dot(norm, direction);
                let diffuse = normal_to_light * light.color.xyz * color * light.color.w / distance;
                let half_dir = normalize(direction + view_ray);
                let spec_angle = max(dot(half_dir, norm), 0.);
                let specular = pow(spec_angle, 16.);
                let spec = specular * light.color.xyz * light.color.w / distance;
                return diffuse + spec;
            }
        }
    }
    return vec3<f32>(0., 0., 0.);
}

fn calculate_lighting(point: vec3<f32>, view_ray: vec3<f32>,norm: vec3<f32>, color: vec3<f32>, epsilon: f32, stack_ptr: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> vec3<f32> {
    var color : vec3<f32> = color * 0.05;
    for (var i : i32 = 0; i < light_settings.num_lights; i = i + 1) {
        let light = lights.lights[i];
        let diffuse = calculate_light(point, view_ray, norm, color, epsilon, light, stack_ptr);
        color = color + diffuse;
    }
    return color;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOut {
    var out : FragmentOut;    
    var stack : array<NodeStackItem, MAX_BRUSH_DEPTH>;
    let stack_pointer : ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>> = &stack;
    let ray = normalize(in.world_position - view.world_position);
    let hit = march(view.world_position + ray * view_extension.near, ray, in.pixel_size, view_extension.far,stack_pointer);
    if (hit.hit) {
        let norm = calculate_normal(hit.point, stack_pointer);
        let color = sceneColor(hit.point, stack_pointer);
        out.color = vec4<f32>(calculate_lighting(hit.point, ray, norm, color, hit.final_epsilon, stack_pointer),1.0);
    } else {
        out.color = vec4<f32>(0., hit.distance / view_extension.far, f32(hit.iterations)/f32(MAX_MARCHING_STEPS), 1.);
    }
        out.depth = 1. - hit.distance / view_extension.far;
    return out;
}

