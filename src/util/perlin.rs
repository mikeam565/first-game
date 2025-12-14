use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use crate::entities::terrain;
pub const WIND_SEED: u32 = 0;
pub const GRASS_HEIGHT_SEED: u32 = 1;
pub const TERRAIN_SEED: u32 = 40658;
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
    // Detail: minimal near BASE_LEVEL (low |mountain_sample|),
    // increases with distance from BASE_LEVEL (both up and down)
    let abs_sample = mountain_sample.abs();
    // Rises quickly then levels off
    let detail_factor = abs_sample.sqrt();
    terrain_perlin.get([z as f64 / 16., x as f64 / 16.]) as f32 * detail_factor * TERRAIN_BUMPINESS
}

fn hill_component(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    let mountain_sample = sample_mountain(terrain_perlin, x, z);
    // Hills: zero near BASE_LEVEL, peak at intermediate elevations
    // (mountain bases), then reduce at peaks. Works for both above and below water.
    let abs_sample = mountain_sample.abs();
    // Bell curve centered at 0.4, multiplied by abs_sample to force zero at origin
    let center = 0.4;
    let width = 0.35;
    let bell = (-((abs_sample - center) / width).powi(2)).exp();
    let hill_factor = bell * abs_sample * 5.0;
    terrain_perlin.get([x as f64 / 100., z as f64 / 100.]) as f32 * hill_factor * HILL_HEIGHTS
}

fn mountain_component(terrain_perlin: &Perlin, x: f32, z: f32) -> f32 {
    let mountain_sample = sample_mountain(terrain_perlin, x, z);
    // No cap - polynomial that accelerates. Works for negative (underwater) too.
    let sign = mountain_sample.signum();
    let abs_sample = mountain_sample.abs();
    // Dead zone only very close to zero for flat areas near BASE_LEVEL
    if abs_sample < 0.05 {
        0.0
    } else {
        let adjusted = abs_sample - 0.05;
        sign * MOUNTAIN_HEIGHTS * adjusted * (1.0 + adjusted * 2.0)
    }
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