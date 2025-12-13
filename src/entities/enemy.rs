
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub const ENEMY_HEIGHT: f32 = 3.0;
const ENEMY_WIDTH: f32 = 1.0;
const MASS: f32 = 10.0;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Enemy {
    name: String
}

#[derive(Component)]
pub struct Health(u8);

pub fn setup_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = Mesh::from(shape::Box::new(ENEMY_WIDTH, ENEMY_HEIGHT, ENEMY_WIDTH));
    let _collider_shape = &ComputedColliderShape::TriMesh;
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(Color::RED),
        transform: Transform::from_xyz(20.0, 200. + ENEMY_HEIGHT + 5., 20.0),
        ..default()
    })
    .insert(RigidBody::Dynamic)
    .insert(Collider::cuboid(ENEMY_WIDTH/2.0,ENEMY_HEIGHT/2.0, ENEMY_WIDTH/2.0))
    .insert(ColliderMassProperties::Mass(MASS))
    // .insert(ActiveEvents::COLLISION_EVENTS)

    .insert(Enemy { name: String::from("BadGuy") })
    .insert(Health(100))
    .insert(Name::new("Enemy"));
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_enemies);
    }
}