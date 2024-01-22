use bevy::prelude::*;
use noise::Perlin;

pub const WIND_SEED: u32 = 0;
pub const GRASS_HEIGHT_SEED: u32 = 1;
pub const TERRAIN_SEED: u32 = 127;

#[derive(Component)]
pub struct PerlinNoiseEntity {
    pub wind: Perlin,
}

impl PerlinNoiseEntity {
    pub fn new() -> Self {
        PerlinNoiseEntity {
            wind: Perlin::new(WIND_SEED),
        }
    }
}

pub fn setup_perlin(mut commands: Commands) {
    commands.spawn(
        PerlinNoiseEntity::new()
    );
}

pub fn grass_perlin() -> Perlin {
    Perlin::new(GRASS_HEIGHT_SEED)
}

pub struct PerlinPlugin;

impl Plugin for PerlinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_perlin);
    }
}