use bevy::{
    math::{Mat4, Vec3, Vec4, Vec4Swizzles},
    prelude::{
        Changed, Commands, CoreStage, Entity, GlobalTransform, Or, Plugin, Query, StageLabel,
        SystemStage, Transform,
    },
};

use crevice::std140::AsStd140;

#[derive(Debug, Clone, PartialEq, Eq, Hash, StageLabel)]
pub enum SDFStages {
    MarkDirty,
    GenerateBounds,
    GenerateGpu,
}

pub struct SDFOperationPlugin;
impl Plugin for SDFOperationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_stage_after(
            CoreStage::Update,
            SDFStages::MarkDirty,
            SystemStage::parallel(),
        )
        .add_stage_after(
            SDFStages::MarkDirty,
            SDFStages::GenerateBounds,
            SystemStage::parallel(),
        )
        .add_stage_after(
            SDFStages::GenerateBounds,
            SDFStages::GenerateGpu,
            SystemStage::parallel(),
        )
        .add_system_to_stage(SDFStages::MarkDirty, mark_dirty_object)
        .add_system_to_stage(SDFStages::GenerateBounds, construct_node_tree_bounds)
        .add_system_to_stage(SDFStages::GenerateGpu, construct_sdf_object_tree);
    }
}

#[derive(Default, Clone, AsStd140)]
pub struct BrushSettings {
    pub num_brushes: i32,
}

#[derive(Debug)]
pub enum SDFShape {
    Sphere(f32),
    Box(f32, f32, f32),
}

impl SDFShape {
    fn process(&self, point: Vec3) -> f32 {
        match self {
            Self::Sphere(radius) => point.length() - radius,
            Self::Box(width, height, depth) => {
                let quadrant = point.abs() - Vec3::new(*width, *height, *depth);
                quadrant.max(Vec3::ZERO).length()
                    + quadrant.x.max(quadrant.y.max(quadrant.z)).min(0.0)
            }
        }
    }
}

#[derive(Debug)]
pub enum SDFOperation {
    Union,
    Subtraction,
    Intersection,
}

impl SDFOperation {
    fn process(&self, a: f32, b: f32, smoothness: f32) -> f32 {
        if smoothness >= 0. {
            match self {
                SDFOperation::Union => a.min(b),
                SDFOperation::Subtraction => a.max(-b),
                SDFOperation::Intersection => a.max(b),
            }
        } else {
            match self {
                SDFOperation::Union => todo!(),
                SDFOperation::Subtraction => todo!(),
                SDFOperation::Intersection => todo!(),
            }
        }
    }
}

#[derive(Debug, AsStd140, Default, Clone)]
pub struct GpuSDFNode {
    pub node_type: i32,
    pub child_a: i32,
    pub child_b: i32,
    pub params: Mat4,
    pub radius: f32,
    pub center: Vec3,
}

#[derive(Debug)]
pub enum SDFNodeData {
    Empty,
    Primitive(SDFShape),
    Operation(SDFOperation, f32, Entity, Entity),
    Transform(Entity),
}

impl Default for SDFNodeData {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug)]
pub struct SDFNode {
    pub data: SDFNodeData,
    pub object: Entity,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SDFGlobalNodeBounds {
    pub radius: f32,
    pub center: Vec3,
}

pub struct SDFObject {
    pub root: Entity,
}

pub struct SDFObjectDirty;

#[derive(Clone, Debug)]
pub struct SDFObjectTree {
    pub tree: Vec<GpuSDFNode>,
}

pub struct SDFRootTransform {
    pub matrix: Mat4,
    pub translation: Vec3,
    pub scale: Vec3,
}

pub const UNION_OP: i32 = 1;
pub const INTERSECTION_OP: i32 = 2;
pub const SUBTRACTION_OP: i32 = 3;
pub const TRANSFORM_WARP: i32 = 4;
pub const SPHERE_PRIM: i32 = 5;
pub const BOX_PRIM: i32 = 6;

pub fn extract_gpu_node_trees(
    mut commands: Commands,
    query: Query<(
        Entity,
        &GlobalTransform,
        &SDFObjectTree,
        &SDFGlobalNodeBounds,
    )>,
) {
    for (entity, transform, tree, bounds) in query.iter() {
        commands
            .get_or_spawn(entity)
            .insert(SDFRootTransform {
                matrix: transform.compute_matrix(),
                translation: transform.translation,
                scale: transform.scale,
            })
            .insert(tree.clone())
            .insert(*bounds);
    }
}

fn generate_node_bounds(
    mut commands: &mut Commands,
    entity: &Entity,
    node_query: &Query<(Entity, &SDFNode, Option<&Transform>)>,
    parent_location: Vec3,
    parent_scale: f32,
) -> SDFGlobalNodeBounds {
    if let Ok((entity, sdfnode, transform)) = node_query.get(entity.to_owned()) {
        let bounds = match &sdfnode.data {
            SDFNodeData::Empty => SDFGlobalNodeBounds::default(),
            SDFNodeData::Primitive(primitive) => match primitive {
                SDFShape::Sphere(radius) => SDFGlobalNodeBounds {
                    center: -parent_location,
                    radius: radius * parent_scale,
                },
                SDFShape::Box(width, height, depth) => SDFGlobalNodeBounds {
                    center: -parent_location,
                    radius: Vec3::new(width.to_owned(), height.to_owned(), depth.to_owned())
                        .length()
                        * parent_scale,
                },
            },
            SDFNodeData::Operation(operation, blending, a, b) => {
                let child_a = generate_node_bounds(
                    &mut commands,
                    a,
                    node_query,
                    parent_location,
                    parent_scale,
                );
                let child_b = generate_node_bounds(
                    &mut commands,
                    b,
                    node_query,
                    parent_location,
                    parent_scale,
                );
                let min_bounds_a = child_a.center - child_a.radius;
                let max_bounds_a = child_a.center + child_a.radius;
                let min_bounds_b = child_b.center - child_b.radius;
                let max_bounds_b = child_b.center + child_b.radius;

                let (min_bounds, max_bounds) = match operation {
                    SDFOperation::Union => (
                        min_bounds_a.min(min_bounds_b),
                        max_bounds_a.max(max_bounds_b),
                    ),
                    SDFOperation::Subtraction => (min_bounds_a, max_bounds_a),
                    SDFOperation::Intersection => (
                        min_bounds_a.max(min_bounds_b),
                        max_bounds_a.min(max_bounds_b),
                    ),
                };

                let center = (max_bounds + min_bounds) / 2.0;
                let extents = (max_bounds - center) + blending.to_owned();
                let radius = extents.max_element();
                SDFGlobalNodeBounds { center, radius }
            }
            SDFNodeData::Transform(child) => {
                let (mut translation, mut scale) = match transform {
                    Some(transform) => (transform.translation, transform.scale.max_element()),
                    None => (Vec3::ZERO, 1.),
                };
                translation += parent_location;
                scale *= parent_scale;

                generate_node_bounds(&mut commands, child, node_query, translation, scale)
            }
        };

        commands.entity(entity).insert(bounds);
        bounds
    } else {
        SDFGlobalNodeBounds::default()
    }
}

fn construct_node_tree_bounds(
    mut commands: Commands,
    object_query: Query<
        (Entity, &SDFObject, &GlobalTransform),
        Or<(Changed<SDFObjectDirty>, Changed<Transform>)>,
    >,
    node_query: Query<(Entity, &SDFNode, Option<&Transform>)>,
) {
    for (entity, object, transform) in object_query.iter() {
        let tree_bounds = generate_node_bounds(
            &mut commands,
            &object.root,
            &node_query,
            transform.translation,
            transform.scale.max_element(),
        );
        commands.entity(entity).insert(tree_bounds);
    }
}

fn generate_gpu_node(
    tree: &mut Vec<GpuSDFNode>,
    entity: &Entity,
    node_query: &Query<(Entity, &SDFNode, &SDFGlobalNodeBounds, Option<&Transform>)>,
) -> (i32, Option<GpuSDFNode>) {
    if let Ok((_entity, sdfnode, bounds, transform)) = node_query.get(entity.to_owned()) {
        let new_id = tree.len();
        tree.push(GpuSDFNode::default());
        let mut new_node = GpuSDFNode {
            center: bounds.center,
            radius: bounds.radius,
            ..Default::default()
        };
        let sdfnode = &sdfnode.data;

        if let SDFNodeData::Primitive(primitive) = sdfnode {
            match primitive {
                SDFShape::Sphere(radius) => {
                    new_node.node_type = SPHERE_PRIM;
                    new_node.params.x_axis.x = radius.to_owned();
                }
                SDFShape::Box(width, height, depth) => {
                    new_node.node_type = BOX_PRIM;
                    new_node.params.x_axis =
                        Vec4::new(width.to_owned(), height.to_owned(), depth.to_owned(), 0.0);
                }
            }
        } else if let SDFNodeData::Operation(operation, blending, child_a, child_b) = sdfnode {
            new_node.node_type = match operation {
                SDFOperation::Union => UNION_OP,
                SDFOperation::Subtraction => SUBTRACTION_OP,
                SDFOperation::Intersection => INTERSECTION_OP,
            };
            new_node.params.x_axis.x = blending.to_owned();
            let (child_a_id, _child_a) = generate_gpu_node(tree, child_a, node_query);
            let (child_b_id, _child_b) = generate_gpu_node(tree, child_b, node_query);
            new_node.child_a = child_a_id - (new_id as i32);
            new_node.child_b = child_b_id - (new_id as i32);
        } else if let SDFNodeData::Transform(child) = sdfnode {
            if let Some(transform) = transform {
                new_node.node_type = TRANSFORM_WARP;
                new_node.params = transform.compute_matrix();
                let (child_id, _child) = generate_gpu_node(tree, child, node_query);
                new_node.child_a = child_id - (new_id as i32);
            }
        }

        tree[new_id] = new_node.clone();

        (new_id as i32, Some(new_node))
    } else {
        println!("Where's my node?");
        (-1, None)
    }
}

pub fn construct_sdf_object_tree(
    mut commands: Commands,
    object_query: Query<(Entity, &SDFObject), Or<(Changed<SDFObjectDirty>, Changed<Transform>)>>,
    node_query: Query<(Entity, &SDFNode, &SDFGlobalNodeBounds, Option<&Transform>)>,
) {
    for (entity, object) in object_query.iter() {
        let mut tree = Vec::<GpuSDFNode>::new();
        generate_gpu_node(&mut tree, &object.root, &node_query);
        commands.entity(entity).insert(SDFObjectTree { tree });
        commands.entity(entity).remove::<SDFObjectDirty>();
    }
}

pub fn mark_dirty_object(mut commands: Commands, query: Query<&SDFNode, Changed<SDFNode>>) {
    for node in query.iter() {
        commands.entity(node.object).insert(SDFObjectDirty);
    }
}

pub fn process_sdf_node(
    point: &Vec3,
    entity: &Entity,
    node_query: &Query<(Entity, &SDFNode, Option<&Transform>)>,
) -> f32 {
    if let Ok((_entity, sdfnode, transform)) = node_query.get(*entity) {
        let sdfnode = &sdfnode.data;
        match sdfnode {
            SDFNodeData::Empty => 9999999.,
            SDFNodeData::Primitive(primitive) => primitive.process(*point),
            SDFNodeData::Operation(op, smoothness, a, b) => {
                let a = process_sdf_node(point, a, node_query);
                let b = process_sdf_node(point, b, node_query);
                op.process(a, b, *smoothness)
            }
            SDFNodeData::Transform(entity) => {
                if let Some(transform) = transform {
                    let point =
                        transform.compute_matrix() * Vec4::new(point.x, point.y, point.z, 1.);
                    process_sdf_node(&point.xyz(), entity, node_query)
                } else {
                    process_sdf_node(point, entity, node_query)
                }
            }
        }
    } else {
        99999999999.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        math::Vec4Swizzles,
        prelude::{ParallelSystemDescriptorCoercion, Stage, SystemStage, World},
    };

    fn assert_eq_f32(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
    }

    #[test]
    fn generate_object_tree() {
        let mut world = World::default();
        let mut bounds_stage = SystemStage::parallel();
        let mut gpu_stage = SystemStage::parallel();
        bounds_stage.add_system(construct_node_tree_bounds.label("sdf_bounds"));
        gpu_stage.add_system(construct_sdf_object_tree.after("sdf_bounds"));

        let object = world.spawn().id();
        let sphere = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Sphere(1.)),
            })
            .id();
        let sphere_transform = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Transform(sphere),
            })
            .insert(Transform::from_translation(Vec3::X))
            .id();
        let cube = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5)),
            })
            .id();
        let union = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Operation(SDFOperation::Union, 0., sphere_transform, cube),
            })
            .id();

        world
            .get_entity_mut(object)
            .unwrap()
            .insert(GlobalTransform::default())
            .insert(SDFObject { root: union })
            .insert(SDFObjectDirty);

        bounds_stage.run(&mut world);
        gpu_stage.run(&mut world);

        let tree = world.get::<SDFObjectTree>(object);
        assert!(tree.is_some());
        if let Some(tree) = tree {
            let tree = &tree.tree;
            assert!(!tree.is_empty());
            let root = &tree[0];
            assert_eq!(root.node_type, UNION_OP);
            assert!(assert_eq_f32(root.params.x_axis.x, 0.));
            assert_eq!(root.center, Vec3::new(-0.5669873, 0., 0.));
            assert!(assert_eq_f32(root.radius, 1.4330127));
            let left_child = &tree[root.child_a as usize];
            let transform_matrix = world
                .get::<Transform>(sphere_transform)
                .unwrap()
                .compute_matrix();
            assert_eq!(left_child.params, transform_matrix);
            assert_eq!(left_child.center, -Vec3::X);
            assert!(assert_eq_f32(left_child.radius, 1.));
            let right_child = &tree[root.child_b as usize];
            assert_eq!(right_child.node_type, BOX_PRIM);
            let right_child_extents = right_child.params.x_axis;
            assert_eq!(right_child_extents.xyz(), Vec3::new(0.5, 0.5, 0.5));
            assert_eq!(right_child.center, Vec3::ZERO);
            assert!(assert_eq_f32(
                right_child.radius,
                (3. * 0.5 * 0.5_f32).sqrt()
            ));
            let sphere = &tree[(root.child_a + left_child.child_a) as usize];
            assert_eq!(sphere.node_type, SPHERE_PRIM);
            assert!(assert_eq_f32(sphere.params.x_axis.x, 1.));
            assert_eq!(sphere.center, -Vec3::X);
            assert!(assert_eq_f32(sphere.radius, 1.));
        }
    }

    #[test]
    fn tree_not_generated_if_object_not_dirty() {
        let mut world = World::default();
        let mut bounds_stage = SystemStage::parallel();
        let mut gpu_stage = SystemStage::parallel();
        bounds_stage.add_system(construct_node_tree_bounds.label("sdf_bounds"));
        gpu_stage.add_system(construct_sdf_object_tree.after("sdf_bounds"));

        let object = world.spawn().id();
        let sphere = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Sphere(1.)),
            })
            .id();
        let sphere_transform = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Transform(sphere),
            })
            .insert(Transform::from_translation(Vec3::X))
            .id();
        let cube = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5)),
            })
            .id();
        let union = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Operation(SDFOperation::Union, 0., sphere_transform, cube),
            })
            .id();

        world
            .get_entity_mut(object)
            .unwrap()
            .insert(GlobalTransform::default())
            .insert(SDFObject { root: union });

        bounds_stage.run(&mut world);
        gpu_stage.run(&mut world);

        let tree = world.get::<SDFObjectTree>(object);
        assert!(tree.is_none());
    }

    #[test]
    fn adding_sdf_node_dirties_object_and_generates_tree() {
        let mut world = World::default();
        let mut update_stage = SystemStage::parallel();
        let mut bounds_stage = SystemStage::parallel();
        let mut gpu_stage = SystemStage::parallel();
        update_stage.add_system(mark_dirty_object);
        bounds_stage.add_system(construct_node_tree_bounds.label("sdf_bounds"));
        gpu_stage.add_system(construct_sdf_object_tree.after("sdf_bounds"));

        let object = world.spawn().id();

        world
            .get_entity_mut(object)
            .unwrap()
            .insert(GlobalTransform::default())
            .insert(SDFObject { root: object });

        update_stage.run(&mut world);
        bounds_stage.run(&mut world);
        gpu_stage.run(&mut world);

        let tree = world.get::<SDFObjectTree>(object);
        assert!(tree.is_none());

        let sphere = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Sphere(1.)),
            })
            .id();
        let sphere_transform = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Transform(sphere),
            })
            .insert(Transform::from_translation(Vec3::X))
            .id();
        let cube = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5)),
            })
            .id();
        let union = world
            .spawn()
            .insert(SDFNode {
                object,
                data: SDFNodeData::Operation(SDFOperation::Union, 0., sphere_transform, cube),
            })
            .id();
        world
            .get_entity_mut(object)
            .unwrap()
            .insert(SDFObject { root: union });

        update_stage.run(&mut world);
        bounds_stage.run(&mut world);
        gpu_stage.run(&mut world);

        let tree = world.get::<SDFObjectTree>(object);
        assert!(tree.is_some());
        if let Some(tree) = tree {
            let tree = &tree.tree;
            assert!(!tree.is_empty());
            let root = &tree[0];
            assert_eq!(root.node_type, UNION_OP);
            assert!(assert_eq_f32(root.params.x_axis.x, 0.));
            assert_eq!(root.center, Vec3::new(-0.5669873, 0., 0.));
            assert!(assert_eq_f32(root.radius, 1.4330127));
            let left_child = &tree[root.child_a as usize];
            let transform_matrix = world
                .get::<Transform>(sphere_transform)
                .unwrap()
                .compute_matrix();
            assert_eq!(left_child.params, transform_matrix);
            assert_eq!(left_child.center, -Vec3::X);
            assert!(assert_eq_f32(left_child.radius, 1.));
            let right_child = &tree[root.child_b as usize];
            assert_eq!(right_child.node_type, BOX_PRIM);
            let right_child_extents = right_child.params.x_axis;
            assert_eq!(right_child_extents.xyz(), Vec3::new(0.5, 0.5, 0.5));
            assert_eq!(right_child.center, Vec3::ZERO);
            assert!(assert_eq_f32(
                right_child.radius,
                (3. * 0.5 * 0.5_f32).sqrt()
            ));
            let sphere = &tree[(root.child_a + left_child.child_a) as usize];
            assert_eq!(sphere.node_type, SPHERE_PRIM);
            assert!(assert_eq_f32(sphere.params.x_axis.x, 1.));
            assert_eq!(sphere.center, -Vec3::X);
            assert!(assert_eq_f32(sphere.radius, 1.));
        }
    }
}
