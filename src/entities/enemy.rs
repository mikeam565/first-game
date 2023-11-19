use std::f32::consts::PI;

use bevy::prelude::*;
use crate::entities as ent;

pub const ENEMY_HEIGHT: f32 = 3.0;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Enemy {
    speed: f32
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
    .insert(Enemy { speed: 5.0 })
    .insert(Health(100))
    .insert(Name::new("Enemy"));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(1.0, ENEMY_HEIGHT, 1.0))),
        material: materials.add(Color::RED.into()),
        transform: Transform::from_xyz(2.5, ENEMY_HEIGHT/2.0, -16.0),
        ..default()
    })
    .insert(Enemy { speed: 0.0 })
    .insert(Health(100))
    .insert(Name::new("Enemy"));
}

fn move_enemies(mut enemies: Query<(&Enemy, &mut Transform)>, time: Res<Time>) {
    for (enemy, mut transform) in &mut enemies {
        transform.translation.z -= enemy.speed * time.delta_seconds();
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_enemies);
        app.add_systems(Update, move_enemies);
    }
}