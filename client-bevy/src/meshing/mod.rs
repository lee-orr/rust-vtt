use bevy::{ render2::*, prelude::*, pbr2::*};

pub struct MeshingPlugin;

impl Plugin for MeshingPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system());
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<bevy::render2::mesh::Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut plane = bevy::render2::mesh::Mesh::new(bevy::render2::render_resource::PrimitiveTopology::TriangleList);
    let v_pos = vec![[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]];
    let v_norm = vec!([0.,1.,0.],[0.,1.,0.],[0.,1.,0.],[0.,1.,0.]);
    
    plane.set_attribute(Mesh::ATTRIBUTE_POSITION, v_pos.clone());
    plane.set_attribute(Mesh::ATTRIBUTE_NORMAL, v_norm);
    plane.set_attribute(Mesh::ATTRIBUTE_UV_0, v_pos.clone());

    let indices = vec![0,2,1,0, 3,2];
    plane.set_indices(Some(bevy::render2::mesh::Indices::U32(indices)));

    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(plane),
        material: materials.add(bevy::render2::color::Color::rgb(0.3,0.3,0.9).into()),
        ..Default::default()
    });
}