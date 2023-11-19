use bevy::prelude::*;
use bevy::utils::FloatOrd;
use crate::entities as ent;
use crate::util;

// constants
const BASIC_PROJECTILE_SPEED: f32 = 20.0;
const BASIC_PROJECTILE_LIFETIME: f32 = 5.0;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct BasicProjectile {
    direction: Vec3,
    speed: f32,
}

pub fn basic_projectile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    enemies: &Query<&GlobalTransform, With<ent::enemy::Enemy>>,
    transform: Transform
) {

    let mesh = meshes.add(
        shape::Icosphere {
            radius: 0.1,
            subdivisions: 5,
        }
        .try_into()
        .unwrap(),
    );

    let direction = match enemies.iter().min_by_key(|enemy_transform| {
        FloatOrd(Vec3::distance(enemy_transform.translation(), transform.translation))
    }).map(|closest_enemy| aim(transform.translation, closest_enemy.translation())) {
            None => Vec3::new(0.0, 0.0, 0.0),
            Some(dir) => { dir }
    };

    commands.spawn(PbrBundle {
        mesh: mesh.clone(),
        material: materials.add(StandardMaterial {
            emissive: Color::rgb_linear(13.99, 5.32, 2.0), // 4. Put something bright in a dark environment to see the effect
            ..default()
        }),
        transform,
        ..default()
    })
    .insert(BasicProjectile { direction, speed: BASIC_PROJECTILE_SPEED })
    .insert(ent::util::Lifetime { timer: Timer::from_seconds(BASIC_PROJECTILE_LIFETIME, TimerMode::Once) })
    .insert(Name::new("Projectile"));
}

fn projectile_movement(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut BasicProjectile, &mut ent::util::Lifetime, &mut Transform)>,
    time: Res<Time>
) {
    for (entity, projectile, mut lifetime, mut transform) in &mut projectiles {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        } else {
            
            let gravity_effect = lifetime.timer.elapsed().as_secs_f32()*util::gravity::GRAVITY_ACC;
            let gravity_vector = util::gravity::GRAVITY_DIR*gravity_effect;

            transform.translation += ((projectile.direction.normalize() * projectile.speed) + gravity_vector) * time.delta_seconds();
        }
    }
}

fn aim(projectile: Vec3, mut dest: Vec3) -> Vec3 {
    // passable gravity compensation for now
    dest.y += ent::enemy::ENEMY_HEIGHT/2.0;
    let distance = projectile.distance(dest);
    let max_distance = 90.0; // highly dependent on the projectile velocity used
    let x = if distance > max_distance { 1.0 } else { distance / max_distance };
    let compensation = Vec3::Y*x;
    let pre_compensation = (dest - projectile).normalize();
    pre_compensation + compensation
}

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, projectile_movement);
    }
}