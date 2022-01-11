use bevy::{
    math::{Vec2, Vec3},
    prelude::{Entity, Plugin},
};

pub struct GridGeneratorPlugin;

impl Plugin for GridGeneratorPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

pub struct Grid {
    pub level: i32,
    pub root_zone: Entity,
}

pub struct GridPoint {
    pub position: Vec2,
    pub zones: Vec<Entity>,
}

pub struct GridContents {
    pub points: Vec<GridPoint>,
}

pub struct GridZoneTriangulation {
    pub zone: Entity,
    pub verticies: Vec<Vec3>,
    pub indices: Vec<i32>,
}
