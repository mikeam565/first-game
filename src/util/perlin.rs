use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use crate::entities::terrain;
pub const WIND_SEED: u32 = 0;
pub const GRASS_HEIGHT_SEED: u32 = 1;
pub const TERRAIN_SEED: u32 = 49;
const HILL_HEIGHTS: f32 = 10.0;
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
    // + terrain_perlin.get([x as f64 / 100., z as f64 / 100.]) as f32 * HILL_HEIGHTS // hills
    // + terrain_perlin.get([z as f64 / 16., x as f64 / 16.]) as f32 * TERRAIN_BUMPINESS // finer detail
    + detail_component(terrain_perlin, x, z)
    + hill_component(terrain_perlin, x, z)
    + mountain_component(terrain_perlin, x, z)
}

fn detail_component(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    let mountain_sample = sample_mountain(terrain_perlin, x, z);
    terrain_perlin.get([z as f64 / 16., x as f64 / 16.]) as f32 * (mountain_sample/0.5)*TERRAIN_BUMPINESS // finer detail
}

fn hill_component(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    let mountain_sample = sample_mountain(terrain_perlin, x, z);

    terrain_perlin.get([x as f64 / 100., z as f64 / 100.]) as f32 * (mountain_sample/0.25)*HILL_HEIGHTS
}

fn mountain_component(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    let mountain_sample = sample_mountain(terrain_perlin, x, z);
    MOUNTAIN_HEIGHTS * mountain_sample/(1.4 - mountain_sample)
}

fn sample_mountain(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    terrain_perlin.get([x as f64 / 4096., z as f64 / 4096.]) as f32
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