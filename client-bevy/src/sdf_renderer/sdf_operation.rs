use std::collections::HashMap;

use bevy::{
    math::{Mat4, Vec3, Vec4},
    prelude::{
        AddAsset, Assets, Changed, Commands, Component, CoreStage, Entity, GlobalTransform, Handle,
        Or, Plugin, Query, Res, StageLabel, SystemStage, Transform,
    },
    reflect::TypeUuid,
    render::{
        render_asset::{RenderAsset, RenderAssetPlugin},
        RenderApp, RenderStage,
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
        app.add_asset::<SDFObjectAsset>()
            .add_plugin(RenderAssetPlugin::<SDFObjectAsset>::default())
            .add_stage_after(
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
            .add_system_to_stage(CoreStage::PreUpdate, clean_dirty_object)
            .add_system_to_stage(SDFStages::MarkDirty, set_dirty_object)
            .add_system_to_stage(SDFStages::GenerateBounds, construct_node_tree_bounds);
        app.sub_app(RenderApp)
            .add_system_to_stage(RenderStage::Extract, extract_gpu_node_trees);
    }
}

#[derive(Default, Clone, Copy, AsStd140)]
pub struct SDFObjectCount {
    pub num_objects: i32,
}

#[derive(Debug, Clone, Copy, Component)]
pub enum SDFShape {
    Sphere(f32),
    Box(f32, f32, f32),
}

#[derive(Debug, Clone, Copy)]
pub enum SDFOperation {
    Union,
    Subtraction,
    Intersection,
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

#[derive(Debug, Copy, Clone)]
pub enum SDFNodeData {
    Empty,
    Primitive(SDFShape),
    Operation(SDFOperation, f32, usize, usize),
    Transform(usize, Transform),
}

impl Default for SDFNodeData {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct SDFGlobalNodeBounds {
    pub radius: f32,
    pub center: Vec3,
}

#[derive(Component)]
pub struct SDFObject {
    pub root: Entity,
}
#[derive(Component)]

pub struct SDFObjectDirty;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "8ecbaccb-e565-4143-ad92-e9a4243bc51e"]
pub struct SDFObjectAsset {
    pub nodes: Vec<(SDFNodeData, Vec3, f32)>,
    pub root: usize,
    pub bounds: (Vec3, f32),
}

impl SDFObjectAsset {
    pub fn new(nodes: Vec<SDFNodeData>) -> Self {
        let mut bound_tree = HashMap::<usize, (SDFNodeData, Vec3, f32)>::new();
        let bounds = generate_node_bounds(0, &nodes, &mut bound_tree);
        Self {
            nodes: { 0..nodes.len() }
                .into_iter()
                .map(|a| {
                    *bound_tree
                        .get(&a)
                        .unwrap_or(&(SDFNodeData::default(), Vec3::ZERO, 0.))
                })
                .collect(),
            root: 0,
            bounds,
        }
    }

    pub fn cube() -> Self {
        Self::new(vec![SDFNodeData::Primitive(SDFShape::Box(1., 1., 1.))])
    }

    pub fn sphere() -> Self {
        Self::new(vec![SDFNodeData::Primitive(SDFShape::Sphere(1.))])
    }

    pub fn test_object(operation: SDFOperation, blend: f32) -> Self {
        Self::new(vec![
            SDFNodeData::Operation(operation, blend, 1, 2),
            SDFNodeData::Primitive(SDFShape::Box(0.2, 0.2, 0.2)),
            SDFNodeData::Transform(3, Transform::from_translation(Vec3::new(2., 0., 0.))),
            SDFNodeData::Primitive(SDFShape::Sphere(2.)),
        ])
    }
}

impl RenderAsset for SDFObjectAsset {
    type ExtractedAsset = Self;

    type PreparedAsset = SDFObjectTree;

    type Param = ();

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        _param: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<
        Self::PreparedAsset,
        bevy::render::render_asset::PrepareAssetError<Self::ExtractedAsset>,
    > {
        let mut tree: Vec<GpuSDFNode> = Vec::new();
        generate_gpu_node(&mut tree, extracted_asset.root, &extracted_asset);
        Ok(Self::PreparedAsset { tree })
    }
}

#[derive(Clone, Debug, Component)]
pub struct SDFObjectTree {
    pub tree: Vec<GpuSDFNode>,
}

#[derive(Component, Debug)]
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
        &Handle<SDFObjectAsset>,
        &SDFGlobalNodeBounds,
    )>,
) {
    let mut count: i32 = 0;
    for (entity, transform, tree, bounds) in query.iter() {
        count += 1;
        let mut ecommands = commands.get_or_spawn(entity);
        ecommands
            .insert(SDFRootTransform {
                matrix: transform.compute_matrix(),
                translation: transform.translation,
                scale: transform.scale,
            })
            .insert(tree.clone())
            .insert(*bounds);
    }
    commands.insert_resource(SDFObjectCount { num_objects: count });
}

pub fn extract_dirty_object(mut commands: Commands, query: Query<(Entity, &SDFObjectDirty)>) {
    for (entity, _) in query.iter() {
        commands.get_or_spawn(entity).insert(SDFObjectDirty);
    }
}

fn generate_node_bounds(
    node_id: usize,
    nodes: &[SDFNodeData],
    bound_nodes: &mut HashMap<usize, (SDFNodeData, Vec3, f32)>,
) -> (Vec3, f32) {
    if let Some(node) = nodes.get(node_id) {
        let (center, radius) = match node {
            SDFNodeData::Empty => (Vec3::ZERO, 0.),
            SDFNodeData::Primitive(primitive) => match primitive {
                SDFShape::Sphere(radius) => (Vec3::ZERO, *radius),
                SDFShape::Box(width, height, depth) => (
                    Vec3::ZERO,
                    Vec3::new(width.to_owned(), height.to_owned(), depth.to_owned()).length(),
                ),
            },
            SDFNodeData::Operation(op, blend, child_a, child_b) => {
                let (center_a, radius_a) = generate_node_bounds(*child_a, nodes, bound_nodes);
                let (center_b, radius_b) = generate_node_bounds(*child_b, nodes, bound_nodes);

                let min_bounds_a = center_a - radius_a;
                let max_bounds_a = center_a + radius_a;
                let min_bounds_b = center_b - radius_b;
                let max_bounds_b = center_b + radius_b;

                let (min, max) = match op {
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
                let center = (max + min) / 2.;
                let extents = (max - center) + *blend;
                (center, extents.length())
            }
            SDFNodeData::Transform(child, transform) => {
                let (center, radius) = generate_node_bounds(*child, nodes, bound_nodes);
                let center = center - transform.translation;
                let radius = radius * transform.scale.max_element();
                (center, radius)
            }
        };

        bound_nodes.insert(node_id, (*node, center, radius));
        (center, radius)
    } else {
        (Vec3::ZERO, 0.)
    }
}

fn construct_node_tree_bounds(
    mut commands: Commands,
    object_query: Query<
        (Entity, &Handle<SDFObjectAsset>, &GlobalTransform),
        Or<(Changed<SDFObjectDirty>, Changed<Transform>)>,
    >,
    objects: Res<Assets<SDFObjectAsset>>,
) {
    for (entity, object, transform) in object_query.iter() {
        let asset = objects.get(object);
        if let Some(object) = asset {
            let bounds = object.bounds;
            let global_bounds = SDFGlobalNodeBounds {
                radius: bounds.1 * transform.scale.max_element(),
                center: bounds.0 - transform.translation,
            };
            commands.entity(entity).insert(global_bounds);
        }
    }
}

fn generate_gpu_node(
    tree: &mut Vec<GpuSDFNode>,
    node: usize,
    node_query: &SDFObjectAsset,
) -> (i32, Option<GpuSDFNode>) {
    if let Some((sdfnode, center, radius)) = node_query.nodes.get(node) {
        let new_id = tree.len();
        tree.push(GpuSDFNode::default());
        let mut new_node = GpuSDFNode {
            center: *center,
            radius: *radius,
            ..Default::default()
        };

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
            let (child_a_id, _child_a) = generate_gpu_node(tree, *child_a, node_query);
            let (child_b_id, _child_b) = generate_gpu_node(tree, *child_b, node_query);
            new_node.child_a = child_a_id - (new_id as i32);
            new_node.child_b = child_b_id - (new_id as i32);
        } else if let SDFNodeData::Transform(child, transform) = sdfnode {
            new_node.node_type = TRANSFORM_WARP;
            new_node.params = transform.compute_matrix();
            let (child_id, _child) = generate_gpu_node(tree, *child, node_query);
            new_node.child_a = child_id - (new_id as i32);
        }

        tree[new_id] = new_node.clone();

        (new_id as i32, Some(new_node))
    } else {
        println!("Where's my node?");
        (-1, None)
    }
}

fn set_dirty_object(
    mut commands: Commands,
    query: Query<(Entity, &Handle<SDFObjectAsset>), Changed<GlobalTransform>>,
    query_2: Query<(Entity, &Handle<SDFObjectAsset>), Changed<Handle<SDFObjectAsset>>>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert(SDFObjectDirty);
    }
    for (entity, _) in query_2.iter() {
        commands.entity(entity).insert(SDFObjectDirty);
    }
}

fn clean_dirty_object(mut commands: Commands, query: Query<(Entity, &SDFObjectDirty)>) {
    for (entity, _) in query.iter() {
        commands.entity(entity).remove::<SDFObjectDirty>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec4Swizzles;

    fn assert_eq_f32(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
    }

    #[test]
    fn generate_object_tree() {
        let object = SDFObjectAsset::new(vec![
            SDFNodeData::Operation(SDFOperation::Union, 0., 1, 3),
            SDFNodeData::Transform(2, Transform::from_translation(Vec3::X)),
            SDFNodeData::Primitive(SDFShape::Sphere(1.)),
            SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5)),
        ]);
        let gpu_object = SDFObjectAsset::prepare_asset(object, &mut ());

        assert!(gpu_object.is_ok());
        if let Ok(tree) = gpu_object {
            println!("GPU OBJECT: {:?}", tree);
            let tree = &tree.tree;
            assert!(!tree.is_empty());
            let root = &tree[0];
            assert_eq!(root.node_type, UNION_OP);
            assert!(assert_eq_f32(root.params.x_axis.x, 0.));
            assert_eq!(root.center, Vec3::new(-0.5669873, 0., 0.));
            assert!(assert_eq_f32(root.radius, 2.013337));
            let left_child = &tree[root.child_a as usize];
            let transform_matrix = Transform::from_translation(Vec3::X).compute_matrix();
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
            assert_eq!(sphere.center, Vec3::ZERO);
            assert!(assert_eq_f32(sphere.radius, 1.));
        }
    }
}
