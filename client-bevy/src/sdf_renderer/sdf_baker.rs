use bevy::{math::Vec3, prelude::{Plugin, Res, ResMut}, render2::{RenderApp, render_resource::TextureView, renderer::RenderDevice, texture::{CachedTexture, TextureCache}}};
use wgpu::{Extent3d, TextureDescriptor, TextureFormat, TextureUsages};

pub struct SDFBakerPlugin;

impl Plugin for SDFBakerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .init_resource::<SDFBakerSettings>()
            .init_resource::<SDFTextures>();
        let render_app = app
            .sub_app(RenderApp)
            .add_startup_system(setup_textures);
    }
}

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