use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::{self, VertexAttributeValues};
use noise::NoiseFn;
use rand::Rng;
use crate::entities::{grass,util};
use crate::util::perlin::{self, sample_terrain_height};
use bevy_rapier3d::prelude::*;

const SUBDIVISIONS: u32 = 100;
const TILE_WIDTH: u32 = 4; // how wide a tile should be
const TEXTURE_SCALE: f32 = 3.;
pub const BASE_LEVEL: f32 = 200.;
pub const WATER_LEVEL: f32 = 195.;

#[derive(Component)]
struct Terrain;

/// set up a simple 3D scene
pub fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let plane_size = if SUBDIVISIONS>0 {TILE_WIDTH*SUBDIVISIONS} else {TILE_WIDTH};
    let num_vertices: usize = (SUBDIVISIONS as usize + 2)*(SUBDIVISIONS as usize + 2);
    let height_map = perlin::terrain_perlin();
    let mut uvs: Vec<[f32;2]> = Vec::with_capacity(num_vertices);
    let mut mesh: Mesh = bevy::prelude::shape::Plane {
        size: plane_size as f32,
        subdivisions: SUBDIVISIONS
    }.into();
    // get positions
    let pos_attr = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap();
    let VertexAttributeValues::Float32x3(pos_attr) = pos_attr else {
        panic!("Unexpected vertex format, expected Float32x3");
    };
    // modify y with height sampling
    for i in 0..pos_attr.len() {
        let pos = pos_attr.get_mut(i).unwrap();
        pos[1] = sample_terrain_height(&height_map, pos[0], pos[2]);
        uvs.push([pos[0]/(TILE_WIDTH as f32*TEXTURE_SCALE), pos[2]/(TILE_WIDTH as f32*TEXTURE_SCALE)]);
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let _ = mesh.generate_tangents();
    
    let sampler_desc = ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..default()
    };
    let settings = move |s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(sampler_desc.clone());
    };

    let texture_handle = asset_server.load_with_settings("terrain/rocky_soil.png", settings.clone());
    let normal_handle = asset_server.load_with_settings("terrain/rocky_soil_normal.png", settings);
    let terrain_material = StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture_handle.clone()),
        normal_map_texture: Some(normal_handle.clone()),
        alpha_mode: AlphaMode::Opaque,
        double_sided: true,
        perceptual_roughness: 1.0,
        reflectance: 0.4,
        cull_mode: Some(Face::Back),
        flip_normal_map_y: true,
        ..default()
    };
    
    // terrain
    let collider_shape = ComputedColliderShape::TriMesh;
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(terrain_material),
        transform: Transform::from_xyz(0.0,0.,0.0),
        ..default()
    })
    .insert(Terrain)
    .insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap());

    // placeholder water plane
    commands.spawn( PbrBundle {
        mesh: meshes.add(shape::Plane {
            size: 1000.,
            subdivisions: 1,
        }.into()),
        material: materials.add(StandardMaterial {
            base_color: Color::BISQUE,
            perceptual_roughness: 0.089,
            diffuse_transmission: 0.7,
            specular_transmission:1.0,
            ..default()
        }),
        transform: Transform::from_xyz(0.0,WATER_LEVEL,0.0),
        ..default()
    });

}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_terrain);
    }
}