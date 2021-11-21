

[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;
[[group(1), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(1), binding(1)]]
var<uniform> brush_settings: BrushSettings;
[[group(2), binding(0)]]
var baked_map: texture_storage_3d<f32, write>;