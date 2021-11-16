use std::primitive;

use bevy::{core::Time, ecs::system::Command, math::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles}, prelude::{Changed, Commands, Entity, GlobalTransform, Handle, IntoSystem, Query, Res, Stage, SystemStage, Transform, World}, render2::render_resource::DynamicUniformVec};
use bytemuck::{Pod, Zeroable};
use crevice::std140::AsStd140;

#[repr(C)]
#[derive(Clone, Copy, Default, AsStd140, Debug)]
pub struct ExtractedSDFBrush {
    shape: i32,
    operation: i32,
    blending: f32,
    transform: Mat4,
    param1: Vec4,
    param2: Vec4
}

#[derive(Debug)]
pub struct ExtractedSDFOrder {
    pub order: u32,
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

#[derive(Debug)]
pub enum SDFOperation {
    Union,
    Subtraction,
    Intersection,
}

pub struct SDFBrush {
    pub order: u32,
    pub shape: SDFShape,
    pub operation: SDFOperation,
    pub blending: f32,
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

#[derive(Debug,)]
pub struct SDFNode {
    data: SDFNodeData,
    object: Entity,
}

pub struct SDFObject {
    pub root: Entity,
}

pub struct SDFObjectDirty;

pub struct SDFObjectTree {
    pub tree: Vec<GpuSDFNode>,
}

const SPHERE_CODE : i32 = 0;
const SQUARE_CODE : i32 = 1;

const UNION_CODE : i32 = 0;
const SUBTRACTION_CODE: i32 = 1;
const INTERSECTION_CODE: i32 = 2;

const UNION_OP : i32 = 1;
const INTERSECTION_OP : i32 = 2;
const SUBTRACTION_OP : i32 = 3;
const TRANSFORM_WARP : i32 = 4;
const SPHERE_PRIM : i32 = 5;
const BOX_PRIM : i32 = 6;

fn extract_sdf_brush(transform: &GlobalTransform, brush: &SDFBrush) -> (ExtractedSDFBrush, ExtractedSDFOrder) {
    let mut extracted = match brush.shape {
        SDFShape::Sphere(radius) => ExtractedSDFBrush { shape: SPHERE_CODE, param1: Vec4::new(radius, 0., 0., 0.), ..Default::default()},
        SDFShape::Box(width, height, depth) => ExtractedSDFBrush{ shape: SQUARE_CODE, param1: Vec4::new(width, height, depth, 0.), ..Default::default()},
    };
    extracted.transform = transform.compute_matrix();
    extracted.blending = brush.blending;
    extracted.operation = match brush.operation {
        SDFOperation::Union => UNION_CODE,
        SDFOperation::Subtraction => SUBTRACTION_CODE,
        SDFOperation::Intersection => INTERSECTION_CODE,
    };
    (extracted, ExtractedSDFOrder { order: brush.order })
}

pub fn extract_sdf_brushes(mut commands: Commands, brushes: Query<(Entity, &GlobalTransform, &SDFBrush)>) {
    for (entity, transform, brush) in brushes.iter() {
        let (sdf, order) = extract_sdf_brush(&transform, &brush);
        commands.get_or_spawn(entity).insert(sdf).insert(order);
    }
}

fn generate_gpu_node(mut tree: &mut Vec<GpuSDFNode>, entity: &Entity, node_query: &Query<(Entity, &SDFNode, Option<&Transform>)>) -> (i32, Option<GpuSDFNode>){
    if let Ok((entity, sdfnode, transform)) = node_query.get(entity.to_owned()) {
        let new_id = tree.len();
        tree.push(GpuSDFNode::default());
        let mut new_node = GpuSDFNode::default();
        let sdfnode = &sdfnode.data;

        if let SDFNodeData::Primitive(primitive) = sdfnode {
            match primitive {
                SDFShape::Sphere(radius) => {
                    new_node.node_type = SPHERE_PRIM;
                    new_node.params.x_axis.x = radius.to_owned();
                    new_node.center = Vec3::ZERO;
                    new_node.radius = radius.to_owned();
                },
                SDFShape::Box(width, height, depth) => {
                    new_node.node_type = BOX_PRIM;
                    new_node.params.x_axis = Vec4::new(width.to_owned(), height.to_owned(), depth.to_owned(), 0.0);
                    new_node.center = Vec3::ZERO;
                    new_node.radius = width.max(height.max(depth.to_owned()));
                },
            }
        } else if let SDFNodeData::Operation(operation, blending, child_a, child_b) = sdfnode {
            new_node.node_type = match operation {
                SDFOperation::Union => UNION_OP,
                SDFOperation::Subtraction => SUBTRACTION_OP,
                SDFOperation::Intersection => INTERSECTION_OP,
            };
            new_node.params.x_axis.x = blending.to_owned();
            let (child_a_id, child_a) = generate_gpu_node(&mut tree, &child_a, &node_query);
            let (child_b_id, child_b) = generate_gpu_node(&mut tree, &child_b, &node_query);
            new_node.child_a = child_a_id;
            new_node.child_b = child_b_id;
            let mut min_bounds = Vec3::ZERO;
            let mut max_bounds = Vec3::ZERO;

            if let Some(child_a) = child_a {
                min_bounds = child_a.center - child_a.radius;
                max_bounds = child_a.center + child_a.radius;
            }
            if let Some(child_b) = child_b {
                min_bounds = min_bounds.min(child_b.center - child_b.radius);
                max_bounds = max_bounds.max(child_b.center + child_b.radius);
            }

            new_node.center = (max_bounds + min_bounds)/2.0;
            let extents = (max_bounds - new_node.center) + blending.to_owned();
            new_node.radius = extents.max_element();
        } else if let SDFNodeData::Transform(child) = sdfnode {
            if let Some(transform) = transform {
                new_node.node_type = TRANSFORM_WARP;
                new_node.params = transform.compute_matrix();
                let (child_id, child) = generate_gpu_node(&mut tree, &child, &node_query);
                new_node.child_a = child_id;
                if let Some(child) = child{
                    new_node.center = child.center + transform.translation;
                    new_node.radius = child.radius * transform.scale.max_element();
                }
            }
        }

        tree[new_id] = new_node.clone();

        (new_id as i32, Some(new_node))
    } else {
        (-1, None)
    }
}

pub fn construct_sdf_object_tree(mut commands: Commands, object_query: Query<(Entity, &SDFObject), Changed<SDFObjectDirty>>, node_query: Query<(Entity, &SDFNode, Option<&Transform>)>) {
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

#[test]
fn generate_object_tree() {
    let mut world = World::default();
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(construct_sdf_object_tree);

    let object = world.spawn().id();
    let sphere = world.spawn().insert(SDFNode { object, data: SDFNodeData::Primitive(SDFShape::Sphere(1.)) }).id();
    let sphere_transform = world.spawn().insert(SDFNode { object, data: SDFNodeData::Transform(sphere)}).insert(Transform::from_translation(Vec3::X)).id();
    let cube = world.spawn().insert(SDFNode { object, data: SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5))}).id();
    let union = world.spawn().insert(SDFNode { object, data: SDFNodeData::Operation(SDFOperation::Union, 0., sphere_transform, cube)}).id();

    world.get_entity_mut(object).unwrap().insert(SDFObject { root: union }).insert(SDFObjectDirty);

    update_stage.run(&mut world);
    
    let tree = world.get::<SDFObjectTree>(object);
    assert!(tree.is_some());
    if let Some(tree) = tree {
        let tree = &tree.tree;
        assert!(tree.len() > 0);
        let root = &tree[0];
        assert_eq!(root.node_type, UNION_OP);
        assert_eq!(root.params.x_axis.x, 0.);
        assert_eq!(root.center, Vec3::new(0.75, 0., 0.));
        assert_eq!(root.radius, 1.25);
        let left_child = &tree[root.child_a as usize];
        let transform_matrix = world.get::<Transform>(sphere_transform).unwrap().compute_matrix();
        assert_eq!(left_child.params, transform_matrix);
        assert_eq!(left_child.center, Vec3::X);
        assert_eq!(left_child.radius, 1.);
        let right_child = &tree[root.child_b as usize];
        assert_eq!(right_child.node_type, BOX_PRIM);
        let right_child_extents = right_child.params.x_axis;
        assert_eq!(right_child_extents.xyz(), Vec3::new(0.5, 0.5, 0.5));
        assert_eq!(right_child.center, Vec3::ZERO);
        assert_eq!(right_child.radius, 0.5);
        let sphere = &tree[left_child.child_a as usize];
        assert_eq!(sphere.params.x_axis.x, 1.);
        assert_eq!(sphere.center, Vec3::ZERO);
        assert_eq!(sphere.radius, 1.);
    }
}

#[test]
fn tree_not_generated_if_object_not_dirty() {
    let mut world = World::default();
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(construct_sdf_object_tree);

    let object = world.spawn().id();
    let sphere = world.spawn().insert(SDFNode { object, data: SDFNodeData::Primitive(SDFShape::Sphere(1.)) }).id();
    let sphere_transform = world.spawn().insert(SDFNode { object, data: SDFNodeData::Transform(sphere)}).insert(Transform::from_translation(Vec3::X)).id();
    let cube = world.spawn().insert(SDFNode { object, data: SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5))}).id();
    let union = world.spawn().insert(SDFNode { object, data: SDFNodeData::Operation(SDFOperation::Union, 0., sphere_transform, cube)}).id();

    world.get_entity_mut(object).unwrap().insert(SDFObject { root: union });

    update_stage.run(&mut world);
    
    let tree = world.get::<SDFObjectTree>(object);
    assert!(tree.is_none());
}

#[test]
fn adding_sdf_node_dirties_object_and_generates_tree() {
    let mut world = World::default();
    let mut update_stage = SystemStage::parallel();
    update_stage.add_system(mark_dirty_object);
    let mut post_update_stage = SystemStage::parallel();
    post_update_stage.add_system(construct_sdf_object_tree);

    let object = world.spawn().id();

    world.get_entity_mut(object).unwrap().insert(SDFObject { root: object });

    update_stage.run(&mut world);
    post_update_stage.run(&mut world);
    
    let tree = world.get::<SDFObjectTree>(object);
    assert!(tree.is_none());
    
    let sphere = world.spawn().insert(SDFNode { object, data: SDFNodeData::Primitive(SDFShape::Sphere(1.)) }).id();
    let sphere_transform = world.spawn().insert(SDFNode { object, data: SDFNodeData::Transform(sphere)}).insert(Transform::from_translation(Vec3::X)).id();
    let cube = world.spawn().insert(SDFNode { object, data: SDFNodeData::Primitive(SDFShape::Box(0.5, 0.5, 0.5))}).id();
    let union = world.spawn().insert(SDFNode { object, data: SDFNodeData::Operation(SDFOperation::Union, 0., sphere_transform, cube)}).id();
    world.get_entity_mut(object).unwrap().insert(SDFObject { root: union });

    update_stage.run(&mut world);
    post_update_stage.run(&mut world);
    
    let tree = world.get::<SDFObjectTree>(object);
    assert!(tree.is_some());
    if let Some(tree) = tree {
        let tree = &tree.tree;
        assert!(tree.len() > 0);
        let root = &tree[0];
        assert_eq!(root.node_type, UNION_OP);
        assert_eq!(root.params.x_axis.x, 0.);
        assert_eq!(root.center, Vec3::new(0.75, 0., 0.));
        assert_eq!(root.radius, 1.25);
        let left_child = &tree[root.child_a as usize];
        let transform_matrix = world.get::<Transform>(sphere_transform).unwrap().compute_matrix();
        assert_eq!(left_child.params, transform_matrix);
        assert_eq!(left_child.center, Vec3::X);
        assert_eq!(left_child.radius, 1.);
        let right_child = &tree[root.child_b as usize];
        assert_eq!(right_child.node_type, BOX_PRIM);
        let right_child_extents = right_child.params.x_axis;
        assert_eq!(right_child_extents.xyz(), Vec3::new(0.5, 0.5, 0.5));
        assert_eq!(right_child.center, Vec3::ZERO);
        assert_eq!(right_child.radius, 0.5);
        let sphere = &tree[left_child.child_a as usize];
        assert_eq!(sphere.params.x_axis.x, 1.);
        assert_eq!(sphere.center, Vec3::ZERO);
        assert_eq!(sphere.radius, 1.);
    }
}