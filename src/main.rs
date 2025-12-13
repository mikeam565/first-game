mod entities;
mod util;

use bevy::{diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, input::InputPlugin, pbr::DirectionalLightShadowMap, prelude::*};
use bevy_atmosphere::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_shader_utils::ShaderUtilsPlugin;
use crate::entities as ent;

fn main() {
    App::new()
        .add_plugins((
            (
                DefaultPlugins,
                ShaderUtilsPlugin,
                WorldInspectorPlugin::new(),
                LogDiagnosticsPlugin::default(),
                FrameTimeDiagnosticsPlugin::default(),
            ),
            ent::poi::PoiPlugin,
            AtmospherePlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            // RapierDebugRenderPlugin::default(),
            util::camera::CameraPlugin,
            util::audio::AudioPlugin,
            util::lighting::LightingPlugin,
            util::perlin::PerlinPlugin,
            ent::terrain::TerrainPlugin,
            ent::grass::GrassPlugin,
            ent::player::PlayerPlugin,
            ent::enemy::EnemyPlugin,
            ent::projectiles::ProjectilePlugin,
        ))
        .register_type::<ent::player::Player>()
        .register_type::<ent::projectiles::BasicProjectile>()
        .run();
}
