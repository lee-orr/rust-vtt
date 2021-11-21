use std::num::NonZeroU32;

use bevy::{math::Vec3, prelude::{Commands, Plugin, Query, Res, ResMut}, render2::{RenderApp, RenderStage, render_resource::TextureView, renderer::{RenderDevice, RenderQueue}, texture::{CachedTexture, TextureCache}}};
use crevice::std430::AsStd430;
use wgpu::{Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, TextureDescriptor, TextureFormat, TextureUsages};

use super::sdf_operation::{SDFObjectTree, SDFRootTransform};

pub struct SDFBakerPlugin;

impl Plugin for SDFBakerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .init_resource::<SDFBakerSettings>();
        let settings = app.world.get_resource::<SDFBakerSettings>();
        let settings = if let Some(settings) = settings {
            println!("Settings are ready!");
            settings.clone()
        } else {
            SDFBakerSettings::default()
        };
        let render_app = app
            .sub_app(RenderApp)
            .insert_resource(settings)
            .init_resource::<SDFTextures>()
            .add_system_to_stage(RenderStage::Prepare, setup_textures)
            .add_system_to_stage(RenderStage::Queue, bake_sdf_texture);
    }
}

#[derive(Clone, Copy)]
pub struct SDFBakerSettings {
    pub max_size: Vec3,
    pub max_depth: u32,
}

impl Default for SDFBakerSettings {
    fn default() -> Self {
        Self {
            max_size: Vec3::new(100., 100., 100.),
            max_depth: 4,
        }
    }
}

#[derive(Default)]
pub struct SDFTextures {
    pub texture: Option<CachedTexture>,
    pub view: Option<TextureView>,
}

const LAYER_SIZE: u32 = 8;

fn setup_textures(settings: Res<SDFBakerSettings>,render_device: Res<RenderDevice>, mut texture_cache: ResMut<TextureCache>, mut textures: ResMut<SDFTextures>) {
    if textures.texture.is_none() {
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Baked SDF"),
                size: Extent3d {
                    depth_or_array_layers: LAYER_SIZE,
                    width: LAYER_SIZE,
                    height: LAYER_SIZE,
                },
                mip_level_count: settings.max_depth,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            }
        );
        let view = texture.default_view.clone();
        textures.texture = Some(texture);
        textures.view = Some(view);
    }
}

fn bake_sdf_texture (settings: Res<SDFBakerSettings>, mut textures: ResMut<SDFTextures>, mut queue: ResMut<RenderQueue>, 
    sdf_roots: Query<(&SDFRootTransform, &SDFObjectTree)>) {
}