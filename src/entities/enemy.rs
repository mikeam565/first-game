use std::f32::consts::PI;

use bevy::prelude::*;
use crate::entities as ent;

pub const ENEMY_HEIGHT: f32 = 3.0;

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
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(1.0, ENEMY_HEIGHT, 1.0))),
        material: materials.add(Color::RED.into()),
        transform: Transform::from_xyz(2.0, ENEMY_HEIGHT/2.0, -1.0),
        ..default()
    })
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