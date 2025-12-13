use bevy::{core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping}, prelude::*, render::view::ColorGrading};
use bevy_atmosphere::prelude::*;

use crate::entities::player::Player;

const VIEW_DISTANCE: f32 = 300000.;

pub fn setup_camera(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    _images: ResMut<Assets<Image>>,
    player: Query<(Entity, &Transform), Added<Player>>
) {
    if !player.is_empty() {
        let (player_entity, _player_transform) = player.get_single().unwrap();
        let cam = commands.spawn((
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
                tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
                projection: bevy::prelude::Projection::Perspective(PerspectiveProjection {
                    far: VIEW_DISTANCE,
                    ..default()
                }),
                transform: Transform::from_xyz(2.8, 2.2, 5.1).with_rotation(Quat::from_rotation_x(0.1)),
                ..default()
            },
            AtmosphereCamera {
                ..default()
            },
            BloomSettings {
                intensity: 0.1,
                // composite_mode: BloomCompositeMode::Additive,
                ..default()
            },
        )).id();
        commands.get_entity(player_entity).unwrap().add_child(cam);
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_camera);
    }
}