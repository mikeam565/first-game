use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh;
use noise::NoiseFn;
use rand::Rng;
use crate::entities::{grass,util};
use crate::util::perlin::{self, sample_terrain_height};
use bevy_rapier3d::prelude::*;

const WIDTH: usize = 64;
const DEPTH: usize = 64;
const TILE_WIDTH: f32 = 4.;

#[derive(Component)]
struct Terrain;

/// set up a simple 3D scene
pub fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
let mut mesh = if util::ENABLE_WIREFRAME {
    Mesh::new(PrimitiveTopology::LineList)
} else {
    Mesh::new(PrimitiveTopology::TriangleList)
};

let num_vertices = WIDTH * DEPTH;
let num_triangles = WIDTH * DEPTH * 2;

let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices);
let height_map = perlin::terrain_perlin();
let mut triangles: Vec<u32> = Vec::with_capacity(num_triangles);
for d in 0..DEPTH {
    for w in 0..WIDTH {
        let w_f32 = w as f32;
        let d_f32 = d as f32;
        
        let pos = [
            w_f32 * TILE_WIDTH,
            sample_terrain_height(&height_map,w_f32 * TILE_WIDTH,d_f32 * TILE_WIDTH),
            d_f32 * TILE_WIDTH
            ];
            positions.push(pos);
            normals.push(pos);
            uvs.push([w_f32 / TILE_WIDTH, d_f32 / TILE_WIDTH]);
        }
    }
    
    for i in 0..positions.capacity() {
        if i%WIDTH!=0 && i/WIDTH < DEPTH - 1 {
            // first triangle
            triangles.push(i as u32);
            triangles.push(i as u32 - 1);
            triangles.push(i as u32 + WIDTH as u32 - 1);
            // second triangle
            triangles.push(i as u32);
            triangles.push(i as u32 + WIDTH as u32 - 1);
            triangles.push(i as u32 + WIDTH as u32);
        }
    }
    
    mesh.set_indices(Some(bevy::render::mesh::Indices::U32(triangles)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.generate_tangents();
    
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
        base_color: Color::BISQUE,
        base_color_texture: Some(texture_handle.clone()),
        normal_map_texture: Some(normal_handle.clone()),
        alpha_mode: AlphaMode::Opaque,
        double_sided: true,
        perceptual_roughness: 1.0,
        reflectance: 0.4,
        cull_mode: Some(Face::Back),
        ..default()
    };
    
    // terrain
    let collider_shape = ComputedColliderShape::TriMesh;
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(terrain_material),
        transform: Transform::from_xyz(0.0,200.,0.0),
        ..default()
    })
    .insert(Terrain)
    .insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap());

}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_terrain);
    }
}