use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::{self, VertexAttributeValues};
use noise::NoiseFn;
use rand::Rng;
use crate::entities::{grass,util,player};
use crate::util::perlin::{self, sample_terrain_height};
use bevy_rapier3d::prelude::*;

// TODO: Design tile/chunk system that doesn't cause tight coupling
struct Tile {
    x: f32,
    z: f32,
    terrain: TerrainBundle,
    grass: GrassBundle,
}