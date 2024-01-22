use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use crate::entities as ent;

const SPEED: f32 = 10.0;
const ROTATION_SPEED: f32 = 0.3;
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

fn player_movement(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut set: ParamSet<(
        Query<(&mut Player, &mut Transform)>,
        Query<(&Camera, &mut Transform)>
    )>,
    enemies: Query<&GlobalTransform, With<ent::enemy::Enemy>>,
    time: Res<Time>
) {
    let mut original_player_pos = Transform::default();
    let mut new_player_pos = Transform::default();
    let mut rotation = 0.;

    for (mut player, mut plyr_trans) in set.p0().iter_mut() {
        original_player_pos = plyr_trans.clone();
        player.shooting_timer.tick(time.delta());
        if player.shooting_timer.just_finished() {
            let mut spawn_transform = plyr_trans
                .with_rotation(Quat::from_rotation_y(-PI / 2.0));
            spawn_transform.translation.y += PLAYER_HEIGHT/2.;

            ent::projectiles::basic_projectile(&mut commands, &mut meshes, &mut materials, &enemies, spawn_transform);
        }
        if keys.pressed(KeyCode::W) {
            let forward = plyr_trans.rotation * -Vec3::Z * SPEED * time.delta_seconds();
            plyr_trans.translation += forward;
        }
        if keys.pressed(KeyCode::S) {
            let forward = plyr_trans.rotation * Vec3::Z * SPEED * time.delta_seconds();
            plyr_trans.translation += forward;
        }
        if keys.pressed(KeyCode::A) {
            let forward = plyr_trans.rotation * -Vec3::X * SPEED * time.delta_seconds();
            plyr_trans.translation += forward;
        }
        if keys.pressed(KeyCode::D) {
            let forward = plyr_trans.rotation * Vec3::X * SPEED * time.delta_seconds();
            plyr_trans.translation += forward;
        }
        if keys.pressed(KeyCode::ShiftLeft) {
            plyr_trans.translation.y += SPEED*time.delta_seconds();
        }
        if keys.pressed(KeyCode::ControlLeft) {
            plyr_trans.translation.y -= SPEED*time.delta_seconds();
        }
        if keys.pressed(KeyCode::Q) {
            rotation = ROTATION_SPEED*TAU*time.delta_seconds();
            plyr_trans.rotate_y(rotation);
        }
        if keys.pressed(KeyCode::E) {
            rotation = -ROTATION_SPEED*TAU*time.delta_seconds();
            plyr_trans.rotate_y(rotation);
        }
        new_player_pos = plyr_trans.clone();
    }
    
    let delta_trans = new_player_pos.translation - original_player_pos.translation;
    let delta_rot = new_player_pos.rotation - original_player_pos.rotation;

    println!("trans:{}, rot:{}", delta_trans, delta_rot);
    
    for (cam, mut cam_trans) in set.p1().iter_mut() {
        if delta_trans != Vec3::ZERO {
            cam_trans.translation += delta_trans;
        }
    
        cam_trans.rotate_around(new_player_pos.translation, Quat::from_rotation_y(rotation));
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, player_movement);
    }
}