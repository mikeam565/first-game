use bevy::{prelude::*, core_pipeline::{tonemapping::Tonemapping, bloom::BloomSettings}};

const CAMERA_SPEED: f32 = 10.0;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 4.0, 12.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            ..default()
        },
        BloomSettings {
            intensity: 0.1,
            // composite_mode: BloomCompositeMode::Additive,
            ..default()
        }
    ));
}

fn update_camera(
    keys: Res<Input<KeyCode>>,
    mut camera: Query<(&Camera, &mut Transform)>,
    time: Res<Time>
) {
    let (cam, mut cam_trans) = camera.get_single_mut().unwrap();
    if keys.pressed(KeyCode::W) {
        cam_trans.translation.z -= CAMERA_SPEED*time.delta_seconds();
    }
    if keys.pressed(KeyCode::S) {
        cam_trans.translation.z += CAMERA_SPEED*time.delta_seconds();
    }
    if keys.pressed(KeyCode::A) {
        cam_trans.translation.x -= CAMERA_SPEED*time.delta_seconds();
    }
    if keys.pressed(KeyCode::D) {
        cam_trans.translation.x += CAMERA_SPEED*time.delta_seconds();
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_camera);
    }
}