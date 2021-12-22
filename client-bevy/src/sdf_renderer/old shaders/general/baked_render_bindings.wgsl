

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;
[[group(1), binding(0)]]
var t_baked : texture_3d<f32>;
[[group(1), binding(1)]]
var s_baked : sampler;
[[group(1), binding(2)]]
var<uniform> baker_settings: SDFBakerSettings;
[[group(1), binding(3)]]
var<uniform> baker_origins: SDFBakedLayerOrigins;
[[group(2), binding(0)]]
var t_depth: texture_depth_2d;
[[group(2), binding(1)]]
var s_depth: sampler;
[[group(2), binding(2)]]
var t_hits: texture_2d<f32>;
[[group(2), binding(3)]]
var s_hits: sampler;