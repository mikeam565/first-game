use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy::utils::FloatOrd;
use crate::entities as ent;
use crate::util;

// constants
const BASIC_PROJECTILE_SPEED: f32 = 80.0;
const BASIC_PROJECTILE_LIFETIME: f32 = 3.0;
const PROJECTILE_RADIUS: f32 = 0.1;
const PROJECTILE_MASS: f32 = 0.2;

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

    let mesh: Mesh = shape::Icosphere {
        radius: PROJECTILE_RADIUS,
        subdivisions: 5,
    }.try_into().unwrap();

    let direction = match enemies.iter().min_by_key(|enemy_transform| {
        FloatOrd(Vec3::distance(enemy_transform.translation(), transform.translation))
    }).map(|closest_enemy| aim(transform.translation, closest_enemy.translation())) {
            None => Vec3::new(0.0, 0.0, 0.0),
            Some(dir) => { dir }
    };

    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(StandardMaterial {
            emissive: Color::rgb_linear(13.99, 5.32, 2.0), // 4. Put something bright in a dark environment to see the effect
            ..default()
        }),
        transform,
        ..default()
    })
    .insert(BasicProjectile)
    .insert(RigidBody::Dynamic)
    .insert(Collider::ball(PROJECTILE_RADIUS))
    .insert(ColliderMassProperties::Mass(PROJECTILE_MASS))
    .insert(Ccd::enabled())
    .insert(Velocity { linvel: direction * BASIC_PROJECTILE_SPEED, angvel: Vec3::ZERO})
    .insert(ent::util::Lifetime { timer: Timer::from_seconds(BASIC_PROJECTILE_LIFETIME, TimerMode::Once) })
    .insert(Name::new("Projectile"));
}

fn projectile_system(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut BasicProjectile, &mut ent::util::Lifetime)>,
    time: Res<Time>
) {
    for (entity, projectile, mut lifetime) in &mut projectiles {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn aim(projectile: Vec3, dest: Vec3) -> Vec3 {
    // passable gravity compensation for now
    let distance = projectile.distance(dest);
    let max_distance = 3000.0; // highly dependent on the projectile velocity used
    let x = if distance > max_distance { 1.0 } else { distance / max_distance };
    let compensation = Vec3::Y*x;
    let pre_compensation = (dest - projectile).normalize();
    pre_compensation + compensation
}

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, projectile_system);
    }
}