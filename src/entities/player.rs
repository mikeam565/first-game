use std::f32::consts::PI;

use bevy::prelude::*;
use crate::entities as ent;

const FIRE_RATE: f32 = 1.0;
const PLAYER_HEIGHT: f32 = 3.0;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Player {
    shooting_timer: Timer
}

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(1.0, PLAYER_HEIGHT, 1.0))),
        material: materials.add(Color::rgb_u8(124, 144, 255).into()),
        transform: Transform::from_xyz(0.0, PLAYER_HEIGHT/2.0, 0.0),
        ..default()
    })
    .insert(Player { shooting_timer: Timer::from_seconds(FIRE_RATE, TimerMode::Repeating)})
    .insert(Name::new("Player"));
}

fn player_shooting(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut players: Query<(&mut Player, &Transform)>,
    enemies: Query<&GlobalTransform, With<ent::enemy::Enemy>>,
    time: Res<Time>
) {
    for (mut player, transform) in &mut players {
        player.shooting_timer.tick(time.delta());
        if player.shooting_timer.just_finished() {
            let spawn_transform = Transform::from_xyz(0.0,PLAYER_HEIGHT,0.0)
                .with_rotation(Quat::from_rotation_y(-PI / 2.0));

            ent::projectiles::basic_projectile(&mut commands, &mut meshes, &mut materials, &enemies, spawn_transform);
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, player_shooting);
    }
}