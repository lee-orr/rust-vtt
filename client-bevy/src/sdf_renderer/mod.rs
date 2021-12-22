pub mod sdf_brush_binding;
mod sdf_depth_binding;
pub mod sdf_lights;
pub mod sdf_object_zones;
pub mod sdf_operation;
pub mod sdf_origin;
pub mod sdf_raw_render_pipeline;
pub mod sdf_raw_with_depth_pass_pipeline;
mod sdf_view_binding;

use bevy::prelude::Plugin;

use crate::sdf_renderer::{
    sdf_object_zones::SDFObjectZonePlugin, sdf_operation::SDFOperationPlugin,
    sdf_origin::SDFOriginPlugin,
};

use self::{
    sdf_brush_binding::SDFBrushBindingPlugin, sdf_lights::SDFLightPlugin,
    sdf_raw_render_pipeline::SDFRawRenderPipelinePlugin, sdf_view_binding::SDFViewBindingPlugin,
};

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(SDFOperationPlugin);
        app.add_plugin(SDFOriginPlugin);
        app.add_plugin(SDFViewBindingPlugin);
        app.add_plugin(SDFBrushBindingPlugin);
        app.add_plugin(SDFLightPlugin);
        app.add_plugin(SDFObjectZonePlugin);
        app.add_plugin(SDFRawRenderPipelinePlugin);
    }
}
