pub mod sdf_object_zones;
pub mod sdf_operation;
pub mod sdf_origin;
pub mod sdf_render_pipeline;

use bevy::prelude::Plugin;

use crate::sdf_renderer::{
    sdf_object_zones::SDFObjectZonePlugin, sdf_operation::SDFOperationPlugin,
    sdf_origin::SDFOriginPlugin,
};

use self::sdf_render_pipeline::SDFRenderPipelinePlugin;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(SDFOperationPlugin);
        app.add_plugin(SDFOriginPlugin);
        app.add_plugin(SDFObjectZonePlugin);
        app.add_plugin(SDFRenderPipelinePlugin);
    }
}
