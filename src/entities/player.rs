use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::{entities as ent, util::{camera::setup_camera, gravity::{GRAVITY_DIR, GRAVITY_ACC}}};

const SPEED: f32 = 15.0;
const ROTATION_SPEED: f32 = 0.3;
const FIRE_RATE: f32 = 0.5;
const PLAYER_HEIGHT: f32 = 3.0;
const PLAYER_WIDTH: f32 = 1.0;
const JUMP_HEIGHT: f32 = 20.0;
const RUN_COEFF: f32 = 3.0;

#[derive(Reflect, Component, Default, Debug)]
#[reflect(Component)]
pub struct Player {
    shooting_timer: Timer
}

pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let transform = Transform::from_xyz(5.0, 200. + PLAYER_HEIGHT + 5., 5.0);
    let mesh = Mesh::from(shape::Box::new(PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH));
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(Color::rgb_u8(124, 144, 255).into()),
        ..default()
    })
    .insert(transform.clone())
    .insert(RigidBody::KinematicPositionBased)
    .insert(Collider::cuboid(PLAYER_WIDTH/2.0, PLAYER_HEIGHT/2.0, PLAYER_WIDTH/2.0))
    .insert(KinematicCharacterController::default())
    .insert(Player { shooting_timer: Timer::from_seconds(FIRE_RATE, TimerMode::Repeating) })
    .insert(Name::new("Player"));

    commands.spawn(setup_camera(transform));
}

fn player_movement(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut set: ParamSet<(
        Query<(&mut Player, &mut Transform, &mut KinematicCharacterController)>,
        Query<(&Camera, &mut Transform)>
    )>,
    enemies: Query<&GlobalTransform, With<ent::enemy::Enemy>>,
    time: Res<Time>
) {
    let base_movement = GRAVITY_ACC*GRAVITY_DIR*time.delta_seconds();
    let mut movement = Vec3::ZERO;

    for (mut player, plyr_trans, mut controller) in set.p0().iter_mut() {
        
        // // shooting
        // player.shooting_timer.tick(time.delta());
        // if player.shooting_timer.just_finished() {
        //     let mut spawn_transform = plyr_trans.clone();
        //     spawn_transform.translation.y += PLAYER_HEIGHT/2. + 1.0;

        //     ent::projectiles::basic_projectile(&mut commands, &mut meshes, &mut materials, &enemies, spawn_transform);
        // }

        // movement
        if keys.pressed(KeyCode::W) {
            movement = movement + plyr_trans.rotation * -Vec3::Z * SPEED * time.delta_seconds();
        }
        if keys.pressed(KeyCode::S) {
            movement = movement + plyr_trans.rotation * Vec3::Z * SPEED * time.delta_seconds();
        }
        if keys.pressed(KeyCode::A) {
            movement = movement + plyr_trans.rotation * -Vec3::X * SPEED * time.delta_seconds();
        }
        if keys.pressed(KeyCode::D) {
            movement = movement + plyr_trans.rotation * Vec3::X * SPEED * time.delta_seconds();
        }

        if keys.pressed(KeyCode::ShiftLeft) {
            movement = movement * RUN_COEFF;
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

// fn read_result_system(controllers: Query<(Entity, &KinematicCharacterControllerOutput)>) {

fn read_result_system(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut set: ParamSet<(
        Query<(&mut Player, &mut Transform, &mut KinematicCharacterControllerOutput)>,
        Query<&mut Transform, With<Camera>>
    )>,
    // enemies: Query<(&mut ent::enemy::Health, &mut ), With<ent::enemy::Enemy>>,
    time: Res<Time>
) {
    let mut new_player_trans = Transform::default();
    let mut effective_trans = Vec3::ZERO;
    let mut rotation = 0.;

    for (player, mut plyr_trans, ctrlr_output) in set.p0().iter_mut() {
        // println!("Entity {:?} moved by {:?} and touches the ground: {:?}", player, ctrlr_output.effective_translation, ctrlr_output.grounded);
        effective_trans = ctrlr_output.effective_translation;
        
        if keys.pressed(KeyCode::Q) {
            rotation += ROTATION_SPEED*TAU*time.delta_seconds();
        }
        if keys.pressed(KeyCode::E) {
            rotation += -ROTATION_SPEED*TAU*time.delta_seconds();
        }
        if rotation != 0. {
            plyr_trans.rotate_y(rotation);
        }

        new_player_trans = plyr_trans.clone();
    }

    for mut cam_trans in set.p1().iter_mut() {
        // for third person
        if effective_trans != Vec3::ZERO {
            cam_trans.translation += effective_trans;
        }
    
        cam_trans.rotate_around(new_player_trans.translation, Quat::from_rotation_y(rotation));
        // for top view
        let looking_dir = new_player_trans.forward() - Vec3::Y*0.2;
        cam_trans.look_to(looking_dir, Vec3::Y);
    }

    
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player);
        app.add_systems(Update, player_movement);
        app.add_systems(Update, read_result_system);
    }
}