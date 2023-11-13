mod entities;
mod util;

use bevy::{prelude::*, pbr::{CascadeShadowConfigBuilder, NotShadowCaster}, core_pipeline::{tonemapping::Tonemapping, bloom::{BloomSettings, BloomCompositeMode}}};
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
            StartupPlugin,
            WorldInspectorPlugin::new(),
            ent::player::PlayerPlugin,
            ent::projectiles::ProjectilePlugin,
            ent::enemy::EnemyPlugin,
            util::camera::CameraPlugin,
            ent::grass::GrassPlugin,
        ))
        .register_type::<ent::player::Player>()
        .register_type::<ent::projectiles::BasicProjectile>()
        .run();
}

pub struct StartupPlugin;

impl Plugin for StartupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            util::perlin::setup_perlin,
            util::lighting::setup_lighting,
            util::camera::setup_camera,
            ent::player::setup_player,
            ent::enemy::setup_enemy,
            ent::terrain::setup_scene,
        ));
    }
}