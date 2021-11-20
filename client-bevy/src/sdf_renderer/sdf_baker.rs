use bevy::{math::Vec3, prelude::Plugin};

pub struct SDFBakerPlugin;

impl Plugin for SDFBakerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SDFBakerSettings>();
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
