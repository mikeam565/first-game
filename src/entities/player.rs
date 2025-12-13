use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::control::KinematicCharacterController;
use noise::NoiseFn;
use crate::util::{gravity::{GRAVITY_ACC, GRAVITY_DIR}, perlin::PerlinNoiseEntity};

const SPEED: f32 = 400.0;
const ROTATION_SPEED: f32 = 0.3;
const FIRE_RATE: f32 = 0.5;
const PLAYER_HEIGHT: f32 = 3.0;
const PLAYER_WIDTH: f32 = 1.0;
const JUMP_HEIGHT: f32 = 20.0;
const RUN_COEFF: f32 = 3.0;
pub const SPAWN_TRANSFORM: Transform = Transform::from_xyz(0.0, 200. + PLAYER_HEIGHT + 5., 0.0);
const TORCH_INTENSITY: f32 = 10_000_000.;
const FLICKER_SPEED: f64 = 2.;
// struct for marking terrain that contains the player
#[derive(Component)]
pub struct ContainsPlayer(pub bool);

#[derive(Reflect, Component, Default, Debug)]
#[reflect(Component)]
pub struct Player {
    shooting_timer: Timer
}

#[derive(Component)]
struct Torch;

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
) {
    let transform = SPAWN_TRANSFORM;
    let mesh = Mesh::from(Cuboid::new(PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH));
    let light = commands.spawn(PointLightBundle {
        point_light: PointLight {
            color: Color::ORANGE,
            intensity: TORCH_INTENSITY,
            ..default()
        },
        transform: Transform::from_xyz(2., 5., 1.),
        ..default()
    }).insert(Torch).id();
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(Color::rgb_u8(124, 144, 255)),
        ..default()
    })
    .insert(transform)
    .insert(RigidBody::KinematicPositionBased)
    .insert(Collider::cuboid(PLAYER_WIDTH/2.0, PLAYER_HEIGHT/2.0, PLAYER_WIDTH/2.0))
    .insert(KinematicCharacterController::default())
    .insert(Player { shooting_timer: Timer::from_seconds(FIRE_RATE, TimerMode::Repeating) })
    .add_child(light)
    .insert(Name::new("Player"));
}

fn player_movement(
    _commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    mut player: Query<(&mut Player, &mut Transform, &mut KinematicCharacterController)>,
    // _enemies: Query<&GlobalTransform, With<ent::enemy::Enemy>>,
    time: Res<Time>
) {
    let base_movement = GRAVITY_ACC*GRAVITY_DIR*time.delta_seconds();
    let mut movement = Vec3::ZERO;
    let mut rotation = 0.;
    if let Ok(player) = player.get_single_mut() {
        let (_player, mut plyr_trans, mut controller) = player;
        // // shooting
        // player.shooting_timer.tick(time.delta());
        // if player.shooting_timer.just_finished() {
        //     let mut spawn_transform = plyr_trans.clone();
        //     spawn_transform.translation.y += PLAYER_HEIGHT/2. + 1.0;

        //     ent::projectiles::basic_projectile(&mut commands, &mut meshes, &mut materials, &enemies, spawn_transform);
        // }

        // movement
        if keys.pressed(KeyCode::KeyW) {
            movement += plyr_trans.rotation * -Vec3::Z * SPEED * time.delta_seconds();
        }
        if keys.pressed(KeyCode::KeyS) {
            movement += plyr_trans.rotation * Vec3::Z * SPEED * time.delta_seconds();
        }
        if keys.pressed(KeyCode::KeyA) {
            movement += plyr_trans.rotation * -Vec3::X * SPEED * time.delta_seconds();
        }
        if keys.pressed(KeyCode::KeyD) {
            movement += plyr_trans.rotation * Vec3::X * SPEED * time.delta_seconds();
        }

        if keys.pressed(KeyCode::ShiftLeft) {
            movement *= RUN_COEFF;
        }

        // rotation
        if keys.pressed(KeyCode::KeyQ) {
            rotation += ROTATION_SPEED*TAU*time.delta_seconds();
        }
        if keys.pressed(KeyCode::KeyE) {
            rotation += -ROTATION_SPEED*TAU*time.delta_seconds();
        }
        if rotation != 0. {
            plyr_trans.rotate_y(rotation);
        }

        // Creative mode flying. Removes gravity effect
        if keys.pressed(KeyCode::ShiftLeft) {
            movement = movement + JUMP_HEIGHT*time.delta_seconds()*Vec3::Y - base_movement;
        }
        if keys.pressed(KeyCode::ControlLeft) {
            movement = movement + JUMP_HEIGHT*time.delta_seconds()*-Vec3::Y - base_movement;
        }

        controller.translation = Some(base_movement + movement);        
    }
}

fn torch_system(
    mut torch_query: Query<&mut PointLight, With<Torch>>,
    perlin: Res<PerlinNoiseEntity>,
    time: Res<Time>,
) {
    if let Ok(mut torch) = torch_query.get_single_mut() {
        let flicker = (1. + perlin.wind.get([time.elapsed_seconds_f64()*FLICKER_SPEED, time.elapsed_seconds_f64()*FLICKER_SPEED]))/2.;
        torch.intensity = TORCH_INTENSITY * flicker as f32;
        
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, (
            player_movement,
            torch_system
        ));
    }
}