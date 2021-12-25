[[group(0), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(0), binding(1)]]
var<uniform> num_objects: SDFObjectCount;

[[group(1), binding(0)]]
var<storage, read_write> tree: Tree;
[[group(1), binding(1)]]
var baked_map: texture_storage_3d<rgba8snorm, read_write>;
[[group(1), binding(2)]]
var<storage, write> TreeDispatchArray;