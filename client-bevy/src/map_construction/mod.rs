use std::f32::consts::PI;

use bevy::{
    math::{EulerRot, Quat, Vec2},
    prelude::{
        BuildChildren, Commands, DespawnRecursiveExt, Entity, Parent, Plugin, Query, Res, ResMut,
        Transform,
    },
};
use bevy_egui::{
    egui::{self, Color32, Ui},
    EguiContext,
};

use self::{
    grid_generator::GridGeneratorPlugin,
    map_zones::{
        BrushBundle, MapZonePlugin, ShapeOperation, Zone, ZoneBoundary, ZoneBrush, ZoneBundle,
        ZoneGrid, ZoneHierarchy, ZoneOrderingId, ZoneShape,
    },
};

pub mod grid_generator;
pub mod map_zones;

pub struct MapConstructionPlugin;

impl Plugin for MapConstructionPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<SelectedZone>()
            .add_system(map_construction_hierarchy)
            .add_system(zone_inspector)
            .add_plugin(MapZonePlugin)
            .add_plugin(GridGeneratorPlugin);
    }
}

#[derive(Debug, Default)]
pub struct SelectedZone {
    pub zone: Option<Entity>,
    pub brush: Option<Entity>,
}

fn zone_hierarchy(
    mut ui: &mut Ui,
    mut commands: &mut Commands,
    mut selected_zone: &mut ResMut<SelectedZone>,
    selected: i32,
    level: &Vec<Entity>,
    hierarchy: &Res<ZoneHierarchy>,
    zones: &Query<(Entity, &Zone, &ZoneOrderingId)>,
) {
    ui.vertical(|ui| {
        let num_items = level.len();
        let max_order = num_items as u32 - 1u32;
        level.iter().enumerate().for_each(|(index, entity)| {
            let entity = *entity;
            let children = hierarchy.child_map.get(&entity);
            if let Ok((entity, zone, _)) = zones.get(entity) {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(selected == entity.id() as i32, &zone.name)
                        .clicked()
                    {
                        selected_zone.zone = Some(entity);
                        selected_zone.brush = None;
                    }
                    if zone.order > 0 && ui.button("Up").clicked() {
                        commands.entity(entity).insert(Zone {
                            name: zone.name.clone(),
                            order: zone.order - 1,
                        });
                        let prev = level[index - 1];
                        if let Ok((prev, zone, _)) = zones.get(prev) {
                            commands.entity(prev).insert(Zone {
                                name: zone.name.clone(),
                                order: zone.order + 1,
                            });
                        }
                    }
                    if zone.order < max_order && ui.button("Down").clicked() {
                        commands.entity(entity).insert(Zone {
                            name: zone.name.clone(),
                            order: zone.order + 1,
                        });
                        let next = level[index + 1];
                        if let Ok((next, zone, _)) = zones.get(next) {
                            commands.entity(next).insert(Zone {
                                name: zone.name.clone(),
                                order: zone.order - 1,
                            });
                        }
                    }
                    if ui.button("New Zone").clicked() {
                        let child = commands.spawn_bundle(ZoneBundle {
                            zone: Zone {
                                name: String::from("Zone"),
                                order: if let Some(children) = children { children.len() as u32 } else { 0 },
                            },
                            ..Default::default()
                        }).id();
                        commands.entity(entity).push_children(&[child]);
                    }
                });
                if let Some(children) = children {
                    ui.collapsing("", |mut ui| {
                        zone_hierarchy(&mut ui, &mut commands, &mut selected_zone, selected, &children, hierarchy, zones);
                    });
                }
            }
        });
    });
}

fn map_construction_hierarchy(
    egui_context: ResMut<EguiContext>,
    mut commands: Commands,
    mut selected_zone: ResMut<SelectedZone>,
    zones: Query<(Entity, &Zone, &ZoneOrderingId)>,
    hierarchy: Option<Res<ZoneHierarchy>>,
) {
    egui::Window::new("Hierarchy").show(egui_context.ctx(), |mut ui| {
        if ui.button("New Zone").clicked() {
            commands.spawn_bundle(ZoneBundle {
                zone: Zone {
                    name: String::from("Zone"),
                    order: if let Some(hierarchy) = &hierarchy { hierarchy.root.len() as u32 } else { 0 },
                },
                ..Default::default()
            });
        }
        let selected = match selected_zone.zone {
            Some(selected) => selected.id() as i32,
            None => -1,
        };
        if let Some(hierarchy) = hierarchy {
            zone_hierarchy(
                &mut ui,
                &mut commands,
                &mut selected_zone,
                selected.clone(),
                &hierarchy.root,
                &hierarchy,
                &zones,
            );
        }
    });
}

fn changed(a: f32, b: f32) -> bool {
    (a - b).abs() > 0.1
}

fn changed_vec(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() > 0.1
}

fn zone_inspector(
    egui_context: ResMut<EguiContext>,
    mut commands: Commands,
    mut selected_zone: ResMut<SelectedZone>,
    zones: Query<(Entity, &Zone, &ZoneGrid, &ZoneBoundary)>,
    mut zone_brushes: Query<(Entity, &mut ZoneBrush, &Parent, &mut Transform)>,
) {
    if let Some(selected) = selected_zone.zone {
        let selected = zones.get(selected);
        if let Ok((selected, zone, _zone_grid, _zone_boundary)) = selected {
            egui::Window::new(&zone.name)
                .id(bevy_egui::egui::Id::new("zone_inspector"))
                .show(egui_context.ctx(), |ui| {
                    let mut name = zone.name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        commands.entity(selected).insert(Zone {
                            name: name.clone(),
                            order: zone.order,
                        });
                    }
                    if ui.button("Remove Zone").clicked() {
                        commands.entity(selected).despawn_recursive();
                    }
                    ui.label("Brushes");
                    let mut num_brushes = 0;
                    let selected_brush = match selected_zone.brush {
                        Some(selected) => selected.id() as i32,
                        None => -1,
                    };
                    zone_brushes.for_each(|(entity, brush, parent, _)| {
                        if parent.0 == selected {
                            num_brushes += 1;
                            let name = brush.shape.name();
                            if ui
                                .selectable_label(selected_brush == entity.id() as i32, name)
                                .clicked()
                            {
                                selected_zone.brush = Some(entity);
                            }
                        }
                    });
                    if ui.button("Add Brush").clicked() {
                        let new_brush = commands
                            .spawn_bundle(BrushBundle::new(selected, num_brushes))
                            .id();
                        commands.entity(selected).push_children(&[new_brush]);
                        selected_zone.brush = Some(new_brush);
                    }
                    if let Some(selected_brush) = selected_zone.brush {
                        let brush = zone_brushes.get_mut(selected_brush);
                        if let Ok((_, mut brush, _, mut transform)) = brush {
                            let frame = egui::Frame {
                                stroke: egui::Stroke::new(1., Color32::BLACK),
                                margin: bevy_egui::egui::Vec2::new(5., 5.),
                                ..Default::default()
                            };
                            frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Shape");
                                    egui::ComboBox::from_id_source("shape_box")
                                        .selected_text(brush.shape.name())
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    matches!(brush.shape, ZoneShape::Circle(_)),
                                                    "circle",
                                                )
                                                .clicked()
                                            {
                                                brush.shape = match brush.shape {
                                                    ZoneShape::Circle(_) => brush.shape,
                                                    _ => ZoneShape::Circle(1.),
                                                };
                                            }
                                            if ui
                                                .selectable_label(
                                                    matches!(brush.shape, ZoneShape::Square(_, _)),
                                                    "square",
                                                )
                                                .clicked()
                                            {
                                                brush.shape = match brush.shape {
                                                    ZoneShape::Square(_, _) => brush.shape,
                                                    _ => ZoneShape::Square(1., 1.),
                                                };
                                            }
                                            if ui
                                                .selectable_label(
                                                    matches!(
                                                        brush.shape,
                                                        ZoneShape::Segment(_, _, _)
                                                    ),
                                                    "segment",
                                                )
                                                .clicked()
                                            {
                                                brush.shape = match brush.shape {
                                                    ZoneShape::Segment(_, _, _) => brush.shape,
                                                    _ => ZoneShape::Segment(
                                                        Vec2::ZERO,
                                                        Vec2::ONE,
                                                        1.,
                                                    ),
                                                };
                                            }
                                            if ui
                                                .selectable_label(
                                                    matches!(
                                                        brush.shape,
                                                        ZoneShape::Curve(_, _, _, _)
                                                    ),
                                                    "curve",
                                                )
                                                .clicked()
                                            {
                                                brush.shape = match brush.shape {
                                                    ZoneShape::Curve(_, _, _, _) => brush.shape,
                                                    _ => ZoneShape::Curve(
                                                        Vec2::ZERO,
                                                        Vec2::ONE,
                                                        2. * Vec2::X,
                                                        1.,
                                                    ),
                                                };
                                            }
                                        });
                                    ui.label("Operation");
                                    egui::ComboBox::from_id_source("operation_box")
                                        .selected_text(brush.operation.name())
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut brush.operation,
                                                ShapeOperation::Union,
                                                "union",
                                            );
                                            ui.selectable_value(
                                                &mut brush.operation,
                                                ShapeOperation::Subtraction,
                                                "subtraction",
                                            );
                                        });
                                    if ui.button("Remove").clicked() {
                                        commands.entity(selected_brush).despawn_recursive();
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Position");
                                    let mut x = transform.translation.x;
                                    let mut y = transform.translation.y;
                                    ui.add(egui::DragValue::new(&mut x).speed(1.));
                                    ui.add(egui::DragValue::new(&mut y).speed(1.));
                                    if changed(x, transform.translation.x)
                                        || changed(y, transform.translation.y)
                                    {
                                        transform.translation.x = x;
                                        transform.translation.y = y;
                                    }
                                    ui.label("Rotation");
                                    let angle =
                                        transform.rotation.to_euler(EulerRot::XYZ).1 * 180. / PI;
                                    let mut mutable_angle = angle;
                                    ui.add(egui::DragValue::new(&mut mutable_angle).speed(1.));
                                    if changed(mutable_angle, angle) {
                                        transform.rotation =
                                            Quat::from_rotation_y(mutable_angle * PI / 180.);
                                    }
                                });
                                match brush.shape {
                                    ZoneShape::Circle(radius) => {
                                        ui.horizontal(|ui| {
                                            ui.label("Radius");
                                            let mut rad = radius;
                                            ui.add(egui::DragValue::new(&mut rad).speed(1.));
                                            if changed(rad, radius) {
                                                brush.shape = ZoneShape::Circle(rad);
                                            }
                                        });
                                    }
                                    ZoneShape::Square(width, height) => {
                                        ui.horizontal(|ui| {
                                            ui.label("Size");
                                            let mut w = width;
                                            let mut h = height;
                                            ui.add(egui::DragValue::new(&mut w).speed(1.));
                                            ui.add(egui::DragValue::new(&mut h).speed(1.));
                                            if changed(w, width) || changed(h, height) {
                                                brush.shape = ZoneShape::Square(w, h);
                                            }
                                        });
                                    }
                                    ZoneShape::Segment(start, end, radius) => {
                                        ui.horizontal(|ui| {
                                            ui.label("Radius");
                                            let mut rad = radius;
                                            ui.add(egui::DragValue::new(&mut rad).speed(1.));
                                            ui.label("Start");
                                            let mut s = start;
                                            ui.add(egui::DragValue::new(&mut s.x).speed(1.));
                                            ui.add(egui::DragValue::new(&mut s.y).speed(1.));
                                            ui.label("End");
                                            let mut e = end;
                                            ui.add(egui::DragValue::new(&mut e.x).speed(1.));
                                            ui.add(egui::DragValue::new(&mut e.y).speed(1.));
                                            if changed(rad, radius)
                                                || changed_vec(s, start)
                                                || changed_vec(e, end)
                                            {
                                                brush.shape = ZoneShape::Segment(s, e, rad);
                                            }
                                        });
                                    }
                                    ZoneShape::Curve(start, end, control, radius) => {
                                        ui.horizontal(|ui| {
                                            ui.label("Radius");
                                            let mut rad = radius;
                                            ui.add(egui::DragValue::new(&mut rad).speed(1.));
                                            ui.label("Start");
                                            let mut s = start;
                                            ui.add(egui::DragValue::new(&mut s.x).speed(1.));
                                            ui.add(egui::DragValue::new(&mut s.y).speed(1.));
                                            ui.label("Control");
                                            let mut c = control;
                                            ui.add(egui::DragValue::new(&mut c.x).speed(1.));
                                            ui.add(egui::DragValue::new(&mut c.y).speed(1.));
                                            ui.label("End");
                                            let mut e = end;
                                            ui.add(egui::DragValue::new(&mut e.x).speed(1.));
                                            ui.add(egui::DragValue::new(&mut e.y).speed(1.));
                                            if changed(rad, radius)
                                                || changed_vec(s, start)
                                                || changed_vec(e, end)
                                                || changed_vec(c, control)
                                            {
                                                brush.shape = ZoneShape::Curve(s, e, c, rad);
                                            }
                                        });
                                    }
                                }
                            });
                        }
                    }
                });
        }
    }
}
