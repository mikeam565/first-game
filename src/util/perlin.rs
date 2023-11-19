use bevy::prelude::*;
use noise::Perlin;

pub const WIND_SEED: u32 = 0;
pub const GRASS_HEIGHT_SEED: u32 = 1;
pub const TERRAIN_SEED: u32 = 127;
#[derive(Component)]
pub struct PerlinNoiseEntity {
    pub wind: Perlin,
    pub grass_height: Perlin,
    pub terrain: Perlin
}

impl PerlinNoiseEntity {
    pub fn new() -> Self {
        PerlinNoiseEntity {
            wind: Perlin::new(WIND_SEED),
            grass_height: Perlin::new(GRASS_HEIGHT_SEED),
            terrain: Perlin::new(TERRAIN_SEED)
        }
    }
}

pub fn setup_perlin(mut commands: Commands) {
    commands.spawn(
        PerlinNoiseEntity::new()
    );
}

pub struct PerlinPlugin;

impl Plugin for PerlinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_perlin);
    }
}