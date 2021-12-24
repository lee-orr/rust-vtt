[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;

[[group(1), binding(0)]]
var<storage, read> tree: Tree;
[[group(1), binding(1)]]
var t_bricks : texture_3d<f32>;
[[group(1), binding(2)]]
var s_bricks : sampler;

[[group(2), binding(0)]]
var<storage, read> lights: Lights;
[[group(2), binding(1)]]
var<uniform> light_settings: LightSettings;