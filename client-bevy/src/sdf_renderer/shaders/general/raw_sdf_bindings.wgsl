
[[group(0), binding(0)]]
var<uniform> view: View;
[[group(0), binding(1)]]
var<uniform> view_extension: ViewExtension;

[[group(1), binding(0)]]
var<storage, read> brushes: Brushes;
[[group(1), binding(1)]]
var<uniform> num_objects: SDFObjectCount;

[[group(2), binding(0)]]
var<storage, read> zones: Zones;
[[group(2), binding(1)]]
var<storage, read> zone_objects: ZoneObjects;
[[group(2), binding(2)]]
var<uniform> num_zones: ZoneSettings;