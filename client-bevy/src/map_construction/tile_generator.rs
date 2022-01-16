use bevy::prelude::*;
use std::collections::HashMap;

use super::map_zones::{
    DirtyZone, GetDistanceField, Zone, ZoneBoundary, ZoneBounds,
    ZoneBrushes, ZoneColor, ZoneHierarchy, ZoneOrderingId,
};

pub struct TileGeneratorPlugin;

impl Plugin for TileGeneratorPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<TileGrid>()
            .init_resource::<TileSettings>()
            .add_system_to_stage(CoreStage::Last, mark_dirt_tiles)
            .add_system_to_stage(CoreStage::First, setup_dirty_tiles)
            .add_system_to_stage(CoreStage::PreUpdate, mesh_tiles);
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DirtyTile;

pub struct TileSettings {
    pub tile_size: f32,
}

impl Default for TileSettings {
    fn default() -> Self {
        Self { tile_size: 1. }
    }
}

#[derive(Component, Default)]
pub struct Tile {
    pub is_boundary: bool,
    pub zones: Vec<ZoneOrderingId>,
}

#[derive(Component, Default)]
pub struct TilePosition {
    position: Vec2,
    level: i32,
    index: (i32, i32, i32),
}

#[derive(Component, Default)]
pub struct TileContents {
    pub contents: Vec<Entity>,
}

#[derive(Debug, Default)]
pub struct TileGrid {
    pub tiles: HashMap<(i32, i32, i32), Entity>,
    pub update: bool,
}

fn mark_dirt_tiles(
    mut commands: Commands,
    tile_settings: Res<TileSettings>,
    mut tile_grid: ResMut<TileGrid>,
    updated_zones: Query<(Entity, &Zone, &ZoneOrderingId, &ZoneBounds), Changed<DirtyZone>>,
    removed_zones: RemovedComponents<Zone>,
) {
    if updated_zones.is_empty() && removed_zones.iter().count() == 0 {
        tile_grid.update = false;
        return;
    }
    tile_grid.update = true;
    let tile_size = tile_settings.tile_size;
    updated_zones.iter().for_each(|(_, zone, _, boundary)| {
        let level = zone.level;
        let min_x = (boundary.min.x / tile_size).floor() as i32;
        let min_y = (boundary.min.y / tile_size).floor() as i32;
        let max_x = (boundary.max.x / tile_size).ceil() as i32;
        let max_y = (boundary.max.y / tile_size).ceil() as i32;
        for x in min_x..max_x {
            for y in min_y..max_y {
                if let Some(tile) = tile_grid.tiles.get(&(x, y, level)) {
                    commands.entity(*tile).insert(DirtyTile);
                } else {
                    let id = commands
                        .spawn()
                        .insert(Tile::default())
                        .insert(TilePosition {
                            position: Vec2::new(x as f32 * tile_size, y as f32 * tile_size),
                            level,
                            index: (x, y, level),
                        })
                        .insert(Transform::from_translation(Vec3::new(
                            x as f32 * tile_size,
                            level as f32 * tile_size,
                            y as f32 * tile_size,
                        )))
                        .insert(GlobalTransform::default())
                        .insert(DirtyTile)
                        .id();
                    tile_grid.tiles.insert((x, y, level), id);
                }
            }
        }
    });
}

fn setup_dirty_tiles(
    mut commands: Commands,
    tile_settings: Res<TileSettings>,
    mut tile_grid: ResMut<TileGrid>,
    tiles: Query<(Entity, &Tile, &TilePosition)>,
    hierarchy: Res<ZoneHierarchy>,
    zone_brushes: Res<ZoneBrushes>,
) {
    if !tile_grid.update {
        return;
    }
    let tile_radius = tile_settings.tile_size / 2.;

    tiles.for_each(|(entity, _tile, position)| {
        println!("processing tile @ {:?}", position.index);
        let mut tile_zones = Vec::<ZoneOrderingId>::new();
        let mut is_boundary = false;
        hierarchy
            .reverse_ordered_zones
            .iter()
            .for_each(|(zone, zone_ordering)| {
                for prev in tile_zones.iter() {
                    if zone_ordering.ancestor_of(prev) {
                        return;
                    }
                }

                if let Some(brushes) = zone_brushes.brushes.get(zone) {
                    let dist = brushes.iter().fold(1000f32, |old, brush| {
                        brush.distance_field(position.position, old)
                    });
                    if dist < tile_radius {
                        tile_zones.push(*zone_ordering);
                        if dist.abs() < tile_radius {
                            is_boundary = true;
                        }
                    }
                }
            });
        if !tile_zones.is_empty() {
            commands.entity(entity).insert(Tile {
                is_boundary,
                zones: tile_zones,
            });
        } else {
            tile_grid.tiles.remove(&position.index);
            commands.entity(entity).despawn_recursive();
        }
    });
}

fn mesh_tiles(
    mut commands: Commands,
    tile_settings: Res<TileSettings>,
    _tile_grid: ResMut<TileGrid>,
    tiles_to_update: Query<(Entity, &Tile, &TilePosition, Option<&TileContents>), Changed<Tile>>,
    _tiles: Query<(Entity, &Tile, &TilePosition)>,
    hierarchy: Res<ZoneHierarchy>,
    zones: Query<(Entity, &Zone, Option<&ZoneColor>, Option<&ZoneBoundary>)>,
    _zone_brushes: Res<ZoneBrushes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let _tile_radius = tile_settings.tile_size / 2.;
    for (entity, tile, _position, contents) in tiles_to_update.iter() {
        if let Some(contents) = contents {
            for item in &contents.contents {
                commands.entity(*item).despawn_recursive();
            }
        }
        // let points = [
        //             position.position + (-Vec2::X + Vec2::Y) * tile_radius,
        //             position.position + (Vec2::X + Vec2::Y) * tile_radius,
        //             position.position + (Vec2::X - Vec2::Y) * tile_radius,
        //             position.position + (-Vec2::X - Vec2::Y) * tile_radius,
        //         ];

        // for zone in &tile.zones {
        //     if let Some(zone) = hierarchy.zone_by_order_id.get(zone) {
        //         if let Some(brushes) = zone_brushes.brushes.get(zone) {
        //             let distances: Vec<f32> = points
        //                 .iter()
        //                 .map(|point| {
        //                     brushes
        //                         .iter()
        //                         .fold(5f32, |old, brush| brush.distance_field(*point, old))
        //                 })
        //                 .collect();
        //         }
        //     }
        // }
        if let Some(zone) = tile.zones.first() {
            if let Some(zone) = hierarchy.zone_by_order_id.get(zone) {
                if let Ok((_, _zone, color, _boundary)) = zones.get(*zone) {
                    let color = if let Some(color) = color {
                        color.color
                    } else {
                        Color::rgb(0.5, 0.5, 0.9)
                    };
                    let content = commands
                        .spawn_bundle(PbrBundle {
                            mesh: meshes.add(
                                shape::Box::new(
                                    tile_settings.tile_size,
                                    0.1,
                                    tile_settings.tile_size,
                                )
                                .into(),
                            ),
                            material: materials.add(color.into()),
                            transform: Transform::from_translation(Vec3::ZERO),
                            ..Default::default()
                        })
                        .id();
                    commands
                        .entity(entity)
                        .add_child(content)
                        .insert(TileContents {
                            contents: vec![content],
                        });
                }
            }
        }
    }
}
