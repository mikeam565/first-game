use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::{self, VertexAttributeValues};
use noise::NoiseFn;
use rand::Rng;
use crate::entities::{grass,util,player};
use crate::util::perlin::{self, sample_terrain_height};
use bevy_rapier3d::prelude::*;

pub const PLANE_SIZE: f32 = 100.;
const SUBDIVISIONS_LEVEL_1: u32 = 20;
const SUBDIVISIONS_LEVEL_2: u32 = 5;
const SUBDIVISIONS_LEVEL_3: u32 = 1;
const TILE_WIDTH: u32 = 4; // how wide a tile should be
const TEXTURE_SCALE: f32 = 3.;
pub const BASE_LEVEL: f32 = 200.;
pub const WATER_LEVEL: f32 = 195.;

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
        // spawn chunk with player in it
        spawn_terrain_chunk(&mut commands, &mut meshes, &mut materials, &asset_server, 0., 0., true, SUBDIVISIONS_LEVEL_1);
        // spawn chunks without player in them
        for (dx,dy) in [(1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1)] {
            let calc_dx = dx as f32 * PLANE_SIZE;
            let calc_dy = dy as f32 * PLANE_SIZE;
            spawn_terrain_chunk(&mut commands, &mut meshes, &mut materials, &asset_server, 0. + calc_dx, 0. + calc_dy, false, SUBDIVISIONS_LEVEL_2);
        }
        spawn_water_plane(&mut commands, &mut meshes, &mut materials);
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
        base_color: if contains_player { Color::WHITE } else { Color::RED },
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

fn update_terrain_colors(
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain_no_player: Query<(Entity,&Transform, &Handle<StandardMaterial>), (Without<ContainsPlayer>,With<Terrain>)>,
    terrain_with_player: Query<(Entity,&Transform, &Handle<StandardMaterial>), (With<ContainsPlayer>,With<Terrain>)>,
) {
    terrain_no_player.iter().for_each(|(e,t,mh)| {
        let mat = materials.get_mut(mh).unwrap();
        mat.base_color = Color::RED;
    });

    terrain_with_player.iter().for_each(|(e,t,mh)| {
        let mat = materials.get_mut(mh).unwrap();
        mat.base_color = Color::WHITE;
    })

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
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let _ = mesh.generate_tangents();

    mesh
}

#[derive(Component)]
struct RegenTerrain;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_terrain);
        app.add_systems(PostUpdate, update_terrain_colors);
    }
}