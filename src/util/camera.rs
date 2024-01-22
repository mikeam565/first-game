use bevy::{prelude::*, core_pipeline::{tonemapping::Tonemapping, bloom::BloomSettings}, render::view::ColorGrading};
use bevy_atmosphere::prelude::*;

use crate::entities::player::Player;

const CAMERA_SPEED: f32 = 10.0;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            color_grading: ColorGrading {
                exposure: 1.2,
                post_saturation: 1.5,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 3.0, 6.0)
                .looking_at(Vec3::new(1.,4.,-4.), Vec3::Y), // TODO: Want the camera spawning to be based off the player
            tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            ..default()
        },
        AtmosphereCamera::default(),
        BloomSettings {
            intensity: 0.1,
            // composite_mode: BloomCompositeMode::Additive,
            ..default()
        }
    ));
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
    }
}