use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::{self, VertexAttributeValues};
use noise::NoiseFn;
use rand::Rng;
use crate::entities::{grass,util,player};
use crate::util::perlin::{self, sample_terrain_height};
use bevy_rapier3d::prelude::*;

const PLANE_SIZE: f32 = 100.;
const SUBDIVISIONS: u32 = 100;
const TILE_WIDTH: u32 = 4; // how wide a tile should be
const TEXTURE_SCALE: f32 = 3.;
pub const BASE_LEVEL: f32 = 200.;
pub const WATER_LEVEL: f32 = 195.;

#[derive(Component)]
pub struct Terrain;

#[derive(Component)]
pub struct ContainsPlayer;


/// set up a simple 3D scene
pub fn update_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    terrain_no_player: Query<(Entity,&Transform), Without<ContainsPlayer>>,
    mut terrain_with_player: Query<(Entity,&Transform), With<ContainsPlayer>>,
    player: Query<&Transform, With<player::Player>>,
) {
    if terrain_with_player.is_empty() { // using this as our indicator that terrain needs to be generated
        spawn_terrain_chunk(&mut commands, &mut meshes, &mut materials, &asset_server, 0., 0.);
        spawn_water_plane(&mut commands, &mut meshes, &mut materials);
    } else {
        let (entity, terrain_trans) = terrain_with_player.get_single_mut().unwrap();
        let player_trans = player.get_single().unwrap();
        let mut new_chunk_x = terrain_trans.translation.x;
        let mut new_chunk_z = terrain_trans.translation.z;
        if (player_trans.translation.x - terrain_trans.translation.x).abs() > PLANE_SIZE/2. {
            new_chunk_x = if player_trans.translation.x - terrain_trans.translation.x > 0. { terrain_trans.translation.x + PLANE_SIZE } else { terrain_trans.translation.x - PLANE_SIZE };
        }
        if (player_trans.translation.z - terrain_trans.translation.z).abs() > PLANE_SIZE/2. {
            new_chunk_z = if player_trans.translation.z - terrain_trans.translation.z > 0. { terrain_trans.translation.z + PLANE_SIZE } else { terrain_trans.translation.z - PLANE_SIZE };
        }
        if new_chunk_x != terrain_trans.translation.x || new_chunk_z != terrain_trans.translation.z { // only check and generate terrain if new_chunk_x or new_chunk_z have been overwritten
            // remove ContainsPlayer from terrain that no longer has the player in it
            commands.get_entity(entity).unwrap().remove::<ContainsPlayer>();
            // check if terrain chunk already exists here, insert ContainsPlayer component
            if let Some(existing_terrain) = check_terrain_exists(&terrain_no_player, &new_chunk_x, &new_chunk_z) {
                commands.get_entity(existing_terrain).unwrap().insert(ContainsPlayer);
            } else { // if doesn't exist, spawn it
                spawn_terrain_chunk(&mut commands, &mut meshes, &mut materials, &asset_server, new_chunk_x, new_chunk_z);
            }
        }
    }
}

fn check_terrain_exists(terrain_no_player: &Query<(Entity,&Transform), Without<ContainsPlayer>>, new_x: &f32, new_z: &f32) -> Option<Entity> {
    for (e, trans) in terrain_no_player.iter() {
        if trans.translation.distance(Vec3::new(*new_x, 0., *new_z))<1. {
            return Some(e)
        }
    }
    None
}

fn spawn_terrain_chunk(commands: &mut Commands, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<StandardMaterial>>, asset_server: &Res<AssetServer>, x: f32, z: f32) {
    
    let num_vertices: usize = (SUBDIVISIONS as usize + 2)*(SUBDIVISIONS as usize + 2);
    let height_map = perlin::terrain_perlin();
    let mut uvs: Vec<[f32;2]> = Vec::with_capacity(num_vertices);
    let mut mesh: Mesh = bevy::prelude::shape::Plane {
        size: PLANE_SIZE,
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
        pos[1] = sample_terrain_height(&height_map, x + pos[0], z + pos[2]);
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
        transform: Transform::from_xyz(x,0.,z),
        ..default()
    })
    .insert(Terrain)
    .insert(ContainsPlayer)
    .insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap());
}

fn spawn_water_plane(commands: &mut Commands, meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<StandardMaterial>>) {
        // placeholder water plane
        commands.spawn( PbrBundle {
            mesh: meshes.add(shape::Plane {
                size: 2000.,
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
        // app.add_systems(Startup, setup_terrain);
        app.add_systems(Update, update_terrain);
    }
}