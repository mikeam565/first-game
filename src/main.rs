mod entities;
mod util;

use bevy::{diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, input::InputPlugin, pbr::DirectionalLightShadowMap, prelude::*};
use bevy_atmosphere::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use crate::entities as ent;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    App::new()
        // .insert_resource(AmbientLight {
        //     brightness: 0.5,
        //     color: Color::AZURE,
        //     ..default()
        // })
        .insert_resource(DirectionalLightShadowMap {
            size: 4096
        })
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new(),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            AtmospherePlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
            // RapierDebugRenderPlugin::default(),
            util::camera::CameraPlugin,
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