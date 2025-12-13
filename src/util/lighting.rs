use std::f32::consts::PI;

use bevy::{pbr::{light_consts::lux::AMBIENT_DAYLIGHT, CascadeShadowConfigBuilder, DirectionalLightShadowMap}, prelude::*};
use bevy_atmosphere::{collection::nishita::Nishita, model::AtmosphereModel, system_param::AtmosphereMut};

// TODO: blue moonlit sky at night

pub fn setup_lighting(mut commands: Commands) {
    commands.spawn( setup_sun() );
    commands.spawn( setup_moon() );
    commands.insert_resource(AmbientLight {
        color: Color::rgba(226./255., 237./255., 255./255., 1.0),
        brightness: 1000.
    })
}

fn setup_sun() -> (DirectionalLightBundle,Sun) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 2.0,
        maximum_distance: 10000.0,
        ..default()
    }.build();
    // Sun
    (
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::rgb(0.98, 0.95, 0.82),
                shadows_enabled: true,
                illuminance: 10000.0,
                ..default()
            },
            transform: Transform::from_xyz(0.0,0.0,0.0)
                .looking_at(Vec3::new(0.2,-0.3,1.0), Vec3::Y),
            cascade_shadow_config,
            ..default()
        },
        Sun
    )
}

fn setup_moon() -> (DirectionalLightBundle,Moon) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 2.0,
        maximum_distance: 10000.0,
        ..default()
    }.build();
    // Sun
    (
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::rgb(225./255.,1.,1.),
                shadows_enabled: true,
                illuminance: 10000.0,
                ..default()
            },
            transform: Transform::from_xyz(0.0,0.0,0.0)
                .looking_at(Vec3::new(-0.2,0.3,-1.0), Vec3::Y),
            cascade_shadow_config,
            ..default()
        },
        Moon
    )
}

fn daylight_cycle(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), (With<Sun>,Without<Moon>)>,
    mut moon: Query<(&mut Transform, &mut DirectionalLight), (With<Moon>,Without<Sun>)>,
    mut ambient: ResMut<AmbientLight>,
    mut timer: ResMut<CycleTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = time.elapsed_seconds_wrapped() / 64.0;
        atmosphere.sun_position = Vec3::new(0., t.sin(), t.cos());

        if let Some((mut light_trans, mut directional)) = sun.single_mut().into() {
            light_trans.rotation = Quat::from_rotation_x(-t);
            directional.illuminance = t.sin().max(0.0).powf(2.0) * AMBIENT_DAYLIGHT;
            directional.shadows_enabled = directional.illuminance > 0.0;
            ambient.brightness = t.sin().max(0.0).powf(2.0) * 1000.;
        }

        if let Some((mut light_trans, mut directional)) = moon.single_mut().into() {
            light_trans.rotation = Quat::from_rotation_x(-t+PI);
            directional.illuminance = (-(t.sin())).max(0.0).powf(2.0) * 250.;
            directional.shadows_enabled = directional.illuminance > 0.0;

        }
    }
}

#[derive(Component)]
struct Sun;

#[derive(Component)]
struct Moon;

#[derive(Resource)]
struct CycleTimer(Timer);

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(AtmosphereModel::new(
                Nishita {
                    ..default()
                }
            ))
            .insert_resource(CycleTimer(Timer::new(
                bevy::utils::Duration::from_millis(20_u64),
                TimerMode::Repeating
            )))
            .insert_resource(DirectionalLightShadowMap {
                size: 4096
            });
        app.add_systems(Startup, setup_lighting);
        app.add_systems(Update, daylight_cycle);
    }
}