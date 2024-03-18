use bevy::{core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping, Skybox}, prelude::*, render::{render_resource::{TextureViewDescriptor, TextureViewDimension}, view::ColorGrading}};
use bevy_atmosphere::{prelude::*, skybox};
use bevy_rapier3d::prelude::*;
use bevy::asset::LoadState;

use crate::entities::player::Player;

const CAMERA_SPEED: f32 = 10.0;
const CAMERA_HEIGHT: f32 = 3.0;
const VIEW_DISTANCE: f32 = 300000.;

pub fn setup_camera(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    player: Query<&Transform, Added<Player>>
) {
    if !player.is_empty() {
        let skybox_image: Handle<Image> = asset_server.load("skybox/day.png");
        let player_transform = player.get_single().unwrap();

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
                transform: Transform::from_xyz(10., 205. + CAMERA_HEIGHT*2.0, 24.)
                    .looking_at(player_transform.translation, Vec3::Y),
                tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
                projection: bevy::prelude::Projection::Perspective(PerspectiveProjection {
                    far: VIEW_DISTANCE,
                    ..default()
                }),
                ..default()
            },
            AtmosphereCamera {
                ..default()
            },
            BloomSettings {
                intensity: 0.1,
                // composite_mode: BloomCompositeMode::Additive,
                ..default()
            }
        ));

        commands.spawn((skybox_image));
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_camera);
    }
}