use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::{self, VertexAttributeValues};
use noise::NoiseFn;
use rand::Rng;
use crate::entities::{grass,util,player};
use crate::util::perlin::{self, sample_terrain_height};
use bevy_rapier3d::prelude::*;

pub const PLANE_SIZE: f32 = 3000.;
const SUBDIVISIONS_LEVEL_1: u32 = 512;
const SUBDIVISIONS_LEVEL_2: u32 = 128;
const SUBDIVISIONS_LEVEL_3: u32 = 2;
const TILE_WIDTH: u32 = 4; // how wide a tile should be
const TEXTURE_SCALE: f32 = 7.;
const WATER_TEXTURE_SCALE: f32 = 20.;
pub const BASE_LEVEL: f32 = 200.;
pub const WATER_LEVEL: f32 = 189.;
const WATER_SCROLL_SPEED: f32 = 0.001;

// struct for marking terrain
#[derive(Component)]
pub struct Terrain;

// struct for marking terrain that contains the player
#[derive(Component)]
pub struct ContainsPlayer;

/// set up a simple 3D scene
pub fn update_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut terrain_no_player: Query<(Entity,&mut Transform,&Handle<Mesh>), (Without<ContainsPlayer>,With<Terrain>)>,
    mut terrain_with_player: Query<(Entity,&mut Transform, &Handle<Mesh>), (With<ContainsPlayer>,With<Terrain>)>,
    player: Query<&Transform, (With<player::Player>,Without<Terrain>)>,
) {
    if terrain_with_player.is_empty() { // scene start
        // spawn chunk at player
        let player_trans = player.get_single().unwrap().translation;
        spawn_terrain_chunk(&mut commands, &mut meshes, &mut materials, &asset_server, 0., 0., true, SUBDIVISIONS_LEVEL_1);
        // spawn chunks without player in them
        for (dx,dz) in [(1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1)] {
            let calc_dx = dx as f32 * PLANE_SIZE;
            let calc_dz = dz as f32 * PLANE_SIZE;
            spawn_terrain_chunk(&mut commands, &mut meshes, &mut materials, &asset_server, player_trans.x + calc_dx, player_trans.z + calc_dz, false, SUBDIVISIONS_LEVEL_2);
        }
        spawn_water_plane(&mut commands, &mut meshes, &mut materials, &asset_server);
    } else { // main update logic
        let (entity, terrain_trans, mh) = terrain_with_player.get_single_mut().unwrap();
        let player_trans = player.get_single().unwrap();
        let mut delta: Option<Vec3> = None;

        // determine player triggering terrain refresh
        if (player_trans.translation.x - terrain_trans.translation.x).abs() > PLANE_SIZE/4. || (player_trans.translation.z - terrain_trans.translation.z).abs() > PLANE_SIZE/4. {
            delta = Some(player_trans.translation - terrain_trans.translation);
        }

        // if they have, regenerate the terrain
        if let Some(delta) = delta {
            println!("Player has triggered terrain regeneration");
            regenerate_terrain(&mut commands, &mut meshes, &mut materials, &asset_server, &mut terrain_no_player, &mut terrain_with_player, delta);
        }
    }
}

fn regenerate_terrain(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    terrain_no_player: &mut Query<(Entity,&mut Transform, &Handle<Mesh>), (Without<ContainsPlayer>, With<Terrain>)>,
    terrain_w_player: &mut Query<(Entity,&mut Transform, &Handle<Mesh>), (With<ContainsPlayer>, With<Terrain>)>,
    delta: Vec3
) {
    let collider_shape = ComputedColliderShape::TriMesh;

    // shift over and regen terrain that didn't have the player
    for (no_pl_ent, mut no_pl_trans, mh) in terrain_no_player.iter_mut() {
        no_pl_trans.translation = no_pl_trans.translation + delta;
        no_pl_trans.translation.y = 0.;
        let mesh = meshes.get_mut(mh).unwrap();
        let new_mesh = &mut generate_terrain_mesh(no_pl_trans.translation.x, no_pl_trans.translation.z, SUBDIVISIONS_LEVEL_2);
        *mesh = new_mesh.clone();
        commands.get_entity(no_pl_ent).unwrap().insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap());
    }

    // shift over and regen terrain that does have the player
    for (pl_ent, mut pl_trans, mh) in terrain_w_player.iter_mut() {
        pl_trans.translation = pl_trans.translation + delta;
        pl_trans.translation.y = 0.;
        let mesh = meshes.get_mut(mh).unwrap();
        let new_mesh = &mut generate_terrain_mesh(pl_trans.translation.x, pl_trans.translation.z, SUBDIVISIONS_LEVEL_1);
        *mesh = new_mesh.clone();
        commands.get_entity(pl_ent).unwrap().insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap());
    }
}

fn spawn_terrain_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    x: f32, z: f32,
    contains_player: bool,
    subdivisions: u32
) -> Entity {    
    let num_vertices: usize = (SUBDIVISIONS_LEVEL_1 as usize + 2)*(SUBDIVISIONS_LEVEL_1 as usize + 2);
    let height_map = perlin::terrain_perlin();
    let mut uvs: Vec<[f32;2]> = Vec::with_capacity(num_vertices);
    let mut mesh: Mesh = bevy::prelude::shape::Plane {
        size: PLANE_SIZE,
        subdivisions
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
        base_color: if contains_player { Color::WHITE } else { Color::BISQUE },
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

    let mut binding = commands.spawn(PbrBundle {
            mesh: meshes.add(mesh.clone()),
            material: materials.add(terrain_material),
            transform: Transform::from_xyz(x,0.,z),
            ..default()
        });
    let parent_terrain = binding
        .insert(Terrain)
        .insert(Collider::from_bevy_mesh(&mesh, &collider_shape).unwrap()
    );
    if contains_player { parent_terrain.insert(ContainsPlayer); }
    parent_terrain.id()
    
}

#[derive(Component)]
struct Water;

fn spawn_water_plane(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    let mut water_mesh: Mesh = shape::Plane {
        size: PLANE_SIZE*3.,
        subdivisions: 4,
    }.into();

    let pos_attr = water_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    let VertexAttributeValues::Float32x3(pos_attr) = pos_attr else {
        panic!("Unexpected vertex format, expected Float32x3");
    };

    let water_uvs: Vec<[f32; 2]> = pos_attr.iter().map(|[x,y,z]| { [x / WATER_TEXTURE_SCALE, z / WATER_TEXTURE_SCALE]}).collect();

    water_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, water_uvs);

    let _ = water_mesh.generate_tangents();

    let sampler_desc = ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..default()
    };
    let settings = move |s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(sampler_desc.clone());
    };
    let normal_handle = asset_server.load_with_settings("water_normal.png", settings);
    // placeholder water plane
    commands.spawn( PbrBundle {
        mesh: meshes.add(water_mesh),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.,96./256.,134./256.),
            perceptual_roughness: 0.7,
            metallic: 0.2,
            reflectance: 0.45,
            diffuse_transmission: 0.0,
            specular_transmission:0.3,
            normal_map_texture: Some(normal_handle.clone()),
            flip_normal_map_y: true,
            ..default()
        }),
        transform: Transform::from_xyz(0.0,WATER_LEVEL,0.0),
        ..default()
    }).insert(Water);
}

fn generate_terrain_mesh(x: f32, z: f32, subdivisions: u32) -> Mesh {
    let num_vertices: usize = (subdivisions as usize + 2)*(subdivisions as usize + 2);
    let height_map = perlin::terrain_perlin();
    let mut uvs: Vec<[f32;2]> = Vec::with_capacity(num_vertices);
    let mut mesh: Mesh = bevy::prelude::shape::Plane {
        size: PLANE_SIZE,
        subdivisions
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
        // Todo: Terrain texture should stay in-place as terrain is updated from player traversal
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let _ = mesh.generate_tangents();

    mesh
}

fn update_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut water: Query<(Entity,&Handle<Mesh>), (With<Water>)>,
) {
    let Ok((water_ent, water_mesh_handle)) = water.get_single_mut() else {
        return
    };
    let water_mesh = meshes.get_mut(water_mesh_handle).unwrap();
    let water_uvs = water_mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap();
    let VertexAttributeValues::Float32x2(uv_attr) = water_uvs else {
        panic!("Unexpected vertex format, expected Float32x3");
    };
    for [x,y] in uv_attr.iter_mut() {
        *x = *x + WATER_SCROLL_SPEED;
        *y = *y + WATER_SCROLL_SPEED;
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_terrain);
        app.add_systems(Update, update_water);
    }
}