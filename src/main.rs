mod entities;
mod util;

use bevy::{prelude::*, pbr::{CascadeShadowConfigBuilder, NotShadowCaster}, core_pipeline::{tonemapping::Tonemapping, bloom::{BloomSettings, BloomCompositeMode}}, diagnostic::{LogDiagnosticsPlugin, FrameTimeDiagnosticsPlugin}};
use bevy_atmosphere::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use crate::entities as ent;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    App::new()
        .insert_resource(AmbientLight {
            brightness: 0.5,
            color: Color::AZURE,
            ..default()
        })
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new(),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            AtmospherePlugin,
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