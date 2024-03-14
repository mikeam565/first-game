use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use crate::entities::terrain;
pub const WIND_SEED: u32 = 0;
pub const GRASS_HEIGHT_SEED: u32 = 1;
pub const TERRAIN_SEED: u32 = 2;
const TERRAIN_HEIGHT_SCALE: f32 = 15.0;
const TERRAIN_SAMPLING_SMOOTHNESS: f64 = 100.;
const TERRAIN_BUMPINESS: f32 = 2.0;
const MOUNTAIN_HEIGHTS: f32 = 256.;

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
    terrain::BASE_LEVEL
    + terrain_perlin.get([x as f64 / TERRAIN_SAMPLING_SMOOTHNESS, z as f64 / TERRAIN_SAMPLING_SMOOTHNESS]) as f32 * TERRAIN_HEIGHT_SCALE // hills
    + terrain_perlin.get([z as f64 / 16., x as f64 / 16.]) as f32 * TERRAIN_BUMPINESS // finer detail
    + terrain_perlin.get([z as f64 / 2048., x as f64 / 2048.]) as f32 * MOUNTAIN_HEIGHTS // mountains

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