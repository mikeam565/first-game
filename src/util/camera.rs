use bevy::{prelude::*, core_pipeline::{tonemapping::Tonemapping, bloom::BloomSettings}, render::view::ColorGrading};
use bevy_atmosphere::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::entities::player::Player;

const CAMERA_SPEED: f32 = 10.0;
const CAMERA_HEIGHT: f32 = 6.0;

pub fn setup_camera(transform: Transform) ->
(bevy::prelude::Camera3dBundle, bevy_atmosphere::plugin::AtmosphereCamera, BloomSettings)
{
    (
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
            transform: Transform::from_xyz(10., 205. + CAMERA_HEIGHT*2.0, 24.)
                .looking_at(transform.translation, Vec3::Y), // TODO: Want the camera spawning to be based off the player
            tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            ..default()
        },
        AtmosphereCamera::default(),
        BloomSettings {
            intensity: 0.1,
            // composite_mode: BloomCompositeMode::Additive,
            ..default()
        }
    )
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // app.add_systems(Startup, setup_camera);
    }
}