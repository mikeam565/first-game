use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh;
use rand::Rng;
use crate::entities::grass;
use crate::util::perlin::PerlinNoiseEntity;
use bevy_rapier3d::prelude::*;

const TERRAIN_SCALE: f32 = 100.0;

#[derive(Component)]
struct Terrain;

/// set up a simple 3D scene
pub fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    let mesh = Mesh::from(shape::Plane {size: 200.0, subdivisions: 0});
    let collider_shape = ComputedColliderShape::TriMesh;
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(Color::rgb(59.0/255.0, 48.0/255.0, 17.0/255.0).into()),
        transform: Transform::from_xyz(50.0,0.0,-50.0),
        ..default()
    })
    .insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap());

    // let terrain = asset_server.load("models/terrain/Mountains.gltf#Scene0");
    // // Terrain
    // commands.spawn(SceneBundle {
    //     scene: terrain,
    //     transform: Transform::from_scale(Vec3::new(TERRAIN_SCALE,TERRAIN_SCALE,TERRAIN_SCALE)),
    //     ..default()
    // })
    // .insert(Terrain);
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_terrain);
    }
}