use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy::utils::FloatOrd;
use bevy_rapier3d::rapier::geometry::ColliderBuilder;
use crate::entities as ent;
use crate::entities::enemy;
use crate::util;

// constants
const BASIC_PROJECTILE_SPEED: f32 = 50.0;
const BASIC_PROJECTILE_LIFETIME: f32 = 3.0;
const PROJECTILE_RADIUS: f32 = 0.1;
const PROJECTILE_MASS: f32 = 0.2;
const PROJECTILE_HEIGHT: f32 = 1.0;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct BasicProjectile;

pub fn basic_projectile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    enemies: &Query<&GlobalTransform, With<ent::enemy::Enemy>>,
    transform: Transform
) {
    // direction of projectile
    let target = match enemies.iter().min_by_key(|enemy_transform| {
        FloatOrd(Vec3::distance(enemy_transform.translation(), transform.translation))
    }).map(|closest_enemy| aim(transform.translation, closest_enemy.translation())) {
            None => Vec3::new(0.0, 0.0, 0.0),
            Some(dir) => { dir }
    };

    // two part collider: heavier tip
    let collider_point = Collider::ball(PROJECTILE_RADIUS);
    let mut point_trans = Transform::default();
    point_trans.translation += 1.05*Vec3::Y*PROJECTILE_HEIGHT/2.0;
    let collider_shaft = Collider::capsule_y(PROJECTILE_HEIGHT/2.0, PROJECTILE_RADIUS);
    let shaft_trans = TransformBundle::default();

    // mesh
    let mesh: Mesh = shape::Capsule {
        radius: PROJECTILE_RADIUS,
        ..default()
    }.try_into().unwrap();

    let mut aimed_transform = transform.looking_at(target, Vec3::Y);
    aimed_transform.rotate_local_x(-PI/2.);

    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(StandardMaterial {
            emissive: Color::rgb_linear(13.99, 5.32, 2.0), // 4. Put something bright in a dark environment to see the effect
            ..default()
        }),
        ..default()
    })
    .insert(BasicProjectile)
    .insert(RigidBody::Dynamic)
    .with_children(|children| {
        children.spawn(collider_point)
        .insert(point_trans)
        .insert(ColliderMassProperties::Mass(PROJECTILE_MASS))
        .insert(ColliderMassProperties::Density(3.0))
        .insert(ActiveEvents::COLLISION_EVENTS);
        children.spawn(collider_shaft)
        .insert(shaft_trans)
        .insert(ColliderMassProperties::Mass(PROJECTILE_MASS/5.0));
    })
    .insert(aimed_transform)
    .insert(ColliderMassProperties::Mass(PROJECTILE_MASS))
    .insert(Ccd::enabled())
    .insert(Velocity { linvel: (target-transform.translation).normalize() * BASIC_PROJECTILE_SPEED, angvel: Vec3::ZERO})
    .insert(ent::util::Lifetime { timer: Timer::from_seconds(BASIC_PROJECTILE_LIFETIME, TimerMode::Once) })
    .insert(Name::new("Projectile"));
}

fn projectile_system(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut ent::util::Lifetime), With<BasicProjectile>>,
    mut collision_events: EventReader<CollisionEvent>,
    time: Res<Time>
) {
    let mut entities = vec![];
    for (entity, mut lifetime) in &mut projectiles {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
        else {
            entities.push(entity);
        }
    }

    // WIP: Despawn projectiles on collision (long-term: move collision logic outside of projectiles.rs)
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Stopped(_,_,_) => {
                // println!("Collision event between {:?} and {:?} with flags {:?}", e1, e2, flags);
            },
            CollisionEvent::Started(e1, e2, flags) => {
                println!("Collision event between {:?} and {:?} with flags {:?}", e1, e2, flags);
                for entity in &entities {
                    if e1.index() == entity.index() {
                        commands.entity(*e1).despawn_recursive();
                    } else if e2.index() == entity.index() {
                        commands.entity(*e2).despawn_recursive();
                    }
                }
            },
        }
    }
}

fn aim(projectile: Vec3, dest: Vec3) -> Vec3 {
    println!("Closest enemy identified at {}", dest);
    let compensation = enemy::ENEMY_HEIGHT;
    dest + Vec3::Y*compensation
}

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, projectile_system);
    }
}