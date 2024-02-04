use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub const WIND_SEED: u32 = 0;
pub const GRASS_HEIGHT_SEED: u32 = 1;
pub const TERRAIN_SEED: u32 = 127;
const TERRAIN_HEIGHT_SCALE: f32 = 10.0;
const TERRAIN_SAMPLING_SMOOTHNESS: f64 = 50.;

#[derive(Resource)]
pub struct PerlinNoiseEntity {
    pub wind: Perlin
}

impl PerlinNoiseEntity {
    pub fn new() -> Self {
        PerlinNoiseEntity {
            wind: Perlin::new(WIND_SEED)
        }
    }

}

pub fn sample_terrain_height(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    terrain_perlin.get([x as f64 / TERRAIN_SAMPLING_SMOOTHNESS, z as f64 / TERRAIN_SAMPLING_SMOOTHNESS]) as f32 * TERRAIN_HEIGHT_SCALE
}

pub fn setup_perlin(mut commands: Commands) {
    commands.insert_resource(PerlinNoiseEntity::new());
}

pub fn grass_perlin() -> Perlin {
    Perlin::new(GRASS_HEIGHT_SEED)
}

pub fn terrain_perlin() -> Perlin {
    Perlin::new(TERRAIN_SEED)
}



pub struct PerlinPlugin;

impl Plugin for PerlinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_perlin);
    }
}