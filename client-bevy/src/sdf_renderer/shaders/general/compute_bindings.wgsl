

[[group(0), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(0), binding(1)]]
var<uniform> brush_settings: BrushSettings;
[[group(1), binding(0)]]
var baked_map: texture_storage_3d<r8unorm, write>;
[[group(1), binding(1)]]
var<uniform> bake_settings: SDFBakerSettings;
[[group(1), binding(2)]]
var<uniform> baker_origins: SDFBakedLayerOrigins;