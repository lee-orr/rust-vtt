// Vertex shader
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};

[[block]]
struct ViewExtension {
    view_proj_inverted: mat4x4<f32>;
    proj_inverted: mat4x4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] ray_direction: vec3<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;

[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 5.;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 5.;
    out.clip_position = vec4<f32>(x, y, 0.2, 1.0);
    let view_space_position = view_extension.view_proj_inverted * out.clip_position;
    let world_position = view_extension.proj_inverted * view_space_position;
    let ray = world_position.xyz - view.world_position;
    out.ray_direction = normalize(ray);
    return out;
}


// Fragment shader


let MAX_MARCHING_STEPS = 100;

fn sceneSDF(point: vec3<f32>) -> f32 {
    return length(point) -  0.5;
}

fn march(start: vec3<f32>, ray: vec3<f32>) -> f32 {
    var depth : f32 = 0.5;
    for (var i : i32 = 0; i < MAX_MARCHING_STEPS; i = i + 1) {
        let dist = sceneSDF(start + depth * ray);
        if (dist < 0.1) {
            return 1.0;
        }
        depth = depth + dist;
    }
    return 0.0;
}


[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let inside = march(view.world_position, in.ray_direction);
    return vec4<f32>(inside, in.ray_direction.x, in.ray_direction.z, 1.0);
}