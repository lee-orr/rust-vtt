[[group(0), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(0), binding(1)]]
var<uniform> num_objects: SDFObjectCount;

[[group(1), binding(0)]]
var<storage, read_write> tree: Tree;
[[group(1), binding(1)]]
var baked_map: texture_storage_3d<rgba8snorm, write>;
[[group(1), binding(3)]]
var<uniform> tree_bake_settings: TreeBakeSettings: 
[[group(1), binding(4)]]
var<storage, read_write> laster_start_points: LayerStartPoints;

[[group(2), binding(0)]]
var<storage, write> next_dispatch: TreeIndirectDispatch;