use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh;
use rand::Rng;
use crate::entities::grass;
use crate::util::perlin::PerlinNoiseEntity;

const TERRAIN_SCALE: f32 = 100.0;

#[derive(Component)]
struct Terrain;

/// set up a simple 3D scene
pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {size: 200.0, subdivisions: 0})),
        material: materials.add(Color::BLACK.into()),
        transform: Transform::from_xyz(50.0,0.0,-50.0),
        ..default()
    });

    // // Terrain
    // commands.spawn(SceneBundle {
    //     scene: asset_server.load("models/terrain/Mountains.gltf#Scene0"),
    //     transform: Transform::from_scale(Vec3::new(TERRAIN_SCALE,TERRAIN_SCALE,TERRAIN_SCALE)),
    //     ..default()
    // })
    // .insert(Terrain);

    grass::generate_grass(&mut commands, &mut meshes, &mut materials);
}