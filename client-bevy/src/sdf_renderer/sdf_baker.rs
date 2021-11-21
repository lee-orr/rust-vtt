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
        let render_app = app
            .sub_app(RenderApp)
            .init_resource::<SDFBakerSettings>()
            .init_resource::<SDFTextures>()
            .add_system_to_stage(RenderStage::Extract, extract_settings)
            .add_system_to_stage(RenderStage::Prepare, setup_textures)
            .add_system_to_stage(RenderStage::Queue, bake_sdf_texture);
    }
}

#[derive(Clone, Copy)]
pub struct SDFBakerSettings {
    pub max_size: Vec3,
    pub layer_size: u8,
    pub num_layers: u8,
    pub layer_multiplier: u8,
}

impl Default for SDFBakerSettings {
    fn default() -> Self {
        Self {
            max_size: Vec3::new(100., 100., 100.),
            layer_size: 8,
            num_layers: 6,
            layer_multiplier: 4,
        }
    }
}

#[derive(Default)]
pub struct SDFTextures {
    pub textures: Vec<CachedTexture>,
    pub views: Vec<TextureView>,
}


fn extract_settings(mut commands: Commands, settings: Res<SDFBakerSettings>) {
    commands.insert_resource(settings.clone());
}

fn setup_textures(settings: Res<SDFBakerSettings>,render_device: Res<RenderDevice>, mut texture_cache: ResMut<TextureCache>, mut textures: ResMut<SDFTextures>) {
    let current_len = textures.textures.len();
    let layer_size = settings.layer_size as u32;
    for i in current_len..(settings.num_layers as usize) {
        let texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Baked SDF"),
                size: Extent3d {
                    depth_or_array_layers: layer_size,
                    width: layer_size,
                    height: layer_size,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            }
        );
        let view = texture.default_view.clone();
        textures.textures.push(texture);
        textures.views.push(view);
    }
}

fn bake_sdf_texture (settings: Res<SDFBakerSettings>, mut textures: ResMut<SDFTextures>, mut queue: ResMut<RenderQueue>, 
    sdf_roots: Query<(&SDFRootTransform, &SDFObjectTree)>) {
}