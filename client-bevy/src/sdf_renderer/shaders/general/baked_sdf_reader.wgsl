
let NORM_EPSILON = 0.0005;
let MAX_BRUSH_DEPTH = 10;

let NORM_EPSILON_X = vec3<f32>(NORM_EPSILON, 0.0, 0.0);
let NORM_EPSILON_Y = vec3<f32>(0.0, NORM_EPSILON, 0.0);
let NORM_EPSILON_Z = vec3<f32>(0.0, 0.0, NORM_EPSILON);

fn sceneSDF(point: vec3<f32>, current_epsilon: f32, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>) -> vec2<f32> {
    var dist : f32 = 9999999999.9;
    var num_jumps : f32 = 0.0;
    let num_objects : i32 = i32(brush_settings.num_objects);
    let p : vec4<f32> = vec4<f32>(point.xyz, 1.0);
    var level : i32 = texture_settings.num_levels;
    var level_size: f32 = texture_settings.max_size;
    var origin : vec3<f32> = texture_settings.origin;
    for (var i : i32 = 0; i < num_objects; i = i + 1) {
        var result = processNode(point, i, current_epsilon, stack);
        var brush_dist : f32 = result.x;
        if (dist > brush_dist) {
            num_jumps = result.y;
        }
        dist = min(dist, brush_dist);
    }
    return vec2<f32>(dist, num_jumps);
}

fn sceneColor(point: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.7, 0.2, 0.2);
}

fn calculate_normal(point: vec3<f32>, stack: ptr<function, array<NodeStackItem, MAX_BRUSH_DEPTH>>)-> vec3<f32> {
    var normal = vec3<f32>(
        sceneSDF(point + NORM_EPSILON_X, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_X, NORM_EPSILON, stack).x,
        sceneSDF(point + NORM_EPSILON_Y, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_Y, NORM_EPSILON, stack).x,
        sceneSDF(point + NORM_EPSILON_Z, NORM_EPSILON, stack).x - sceneSDF(point - NORM_EPSILON_Z, NORM_EPSILON, stack).x,
    );
    return normalize(normal);
}