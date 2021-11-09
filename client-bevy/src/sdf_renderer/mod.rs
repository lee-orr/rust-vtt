use bevy::{core_pipeline::{SetItemPipeline, Transparent2d, Transparent3d}, prelude::Plugin, render2::{RenderApp, color::Color, render_phase::{AddRenderCommand, RenderCommand}}};

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.sub_app(RenderApp);
        //    .add_render_command::<Transparent3d, DrawSDF>()
        //    .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}
/*
type DrawSDF = (SetItemPipeline, SetSDFBindGroup);

struct SetSDFBindGroup;
impl RenderCommand<Transparent3d> for SetSDFBindGroup {
    type Param = (SRes<);

    fn render<'w>(
        view: bevy::prelude::Entity,
        item: &Transparent3d,
        param: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render2::render_phase::TrackedRenderPass<'w>,
    ) {
        pass.set_bind_group(index, bind_group, dynamic_uniform_indices)
    }
} */