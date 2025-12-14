use bevy::render::texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::{prelude::*, render::render_resource::Face};
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::mesh::Indices;
use bevy::render::primitives::Frustum;
use bevy::utils::HashMap;
use bevy::ecs::system::{CommandQueue, SystemState};
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use futures_lite::future::poll_once;
use crate::entities::player;
use crate::util::perlin::{self, sample_terrain_height};
use crate::util::render_state::{RenderState, FrustumHidden};
use bevy_rapier3d::prelude::*;

// Chunk configuration
pub const CHUNK_SIZE: f32 = 512.; // Size of each terrain chunk
const CHUNKS_RADIUS: i32 = 12; // How many chunks in each direction from player

// LOD levels - subdivisions decrease with distance
const LOD_0_SUBDIVISIONS: u32 = 64; // Highest detail (close to player)
const LOD_1_SUBDIVISIONS: u32 = 32;
const LOD_2_SUBDIVISIONS: u32 = 16;
const LOD_3_SUBDIVISIONS: u32 = 8; // Lowest detail (far from player)

// Distance thresholds for LOD transitions (in chunks)
const LOD_1_DISTANCE: i32 = 3;
const LOD_2_DISTANCE: i32 = 6;
const LOD_3_DISTANCE: i32 = 9;

// Distance threshold for physics colliders (in chunks) - needs to be large enough
// to ensure colliders exist before the player reaches them
const COLLIDER_DISTANCE: i32 = 2;

// Maximum number of terrain chunks to spawn per frame (rate limiting)
const MAX_CHUNKS_PER_FRAME: usize = 8;

const TILE_WIDTH: u32 = 16; // how wide a tile should be
const TEXTURE_SCALE: f32 = 7.;
const WATER_TEXTURE_SCALE: f32 = 20.;
pub const BASE_LEVEL: f32 = 200.;
pub const WATER_LEVEL: f32 = 189.;
const WATER_SCROLL_SPEED: f32 = 0.0002;
const HEIGHT_PEAKS: f32 = 1500.;
const HEIGHT_SAND: f32 = 200.;
pub const HEIGHT_TEMPERATE_START: f32 = 210.;
pub const HEIGHT_TEMPERATE_END: f32 = 800.;
const COLOR_TEMPERATE: [f32;4] = [0.079,0.079,0.,1.];
const COLOR_SAND: [f32;4] = [80./255., 72./255., 49./255., 255./255.];
const COLOR_PEAKS: [f32;4] = [255./255.,255./255.,255./255.,255./255.];

/// Margin for frustum culling - accounts for terrain height variations
const FRUSTUM_CULLING_MARGIN: f32 = CHUNK_SIZE * 1.5;

// Terrain chunk component
#[derive(Component)]
pub struct Terrain {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub lod_level: u32,
}

/// Terrain-specific chunk state that wraps the unified RenderState
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TerrainChunkState {
    pub render_state: RenderState,
    pub lod_level: u32,
}

impl TerrainChunkState {
    pub fn visible(lod_level: u32) -> Self {
        Self { render_state: RenderState::Visible, lod_level }
    }

    pub fn hidden(lod_level: u32) -> Self {
        Self { render_state: RenderState::Hidden, lod_level }
    }

    pub fn pending() -> Self {
        Self { render_state: RenderState::Pending, lod_level: 0 }
    }
}

#[derive(Component)]
pub struct TerrainGrid(pub HashMap<(i32, i32), TerrainChunkState>);

// Component to mark chunk with physics collider (only close chunks get colliders)
#[derive(Component)]
pub struct TerrainCollider;

// Component for async terrain generation task
#[derive(Component)]
struct GenTerrainTask(Task<CommandQueue>);

/// Calculate the LOD level based on chunk distance from player
fn get_lod_level(chunk_distance: i32) -> u32 {
    if chunk_distance < LOD_1_DISTANCE {
        LOD_0_SUBDIVISIONS
    } else if chunk_distance < LOD_2_DISTANCE {
        LOD_1_SUBDIVISIONS
    } else if chunk_distance < LOD_3_DISTANCE {
        LOD_2_SUBDIVISIONS
    } else {
        LOD_3_SUBDIVISIONS
    }
}

/// Check if a point is inside the camera frustum with a margin
fn is_chunk_in_frustum(frustum: &Frustum, chunk_center: Vec3) -> bool {
    for half_space in &frustum.half_spaces {
        if half_space.normal_d().dot(chunk_center.extend(1.0)) < -FRUSTUM_CULLING_MARGIN {
            return false;
        }
    }
    true
}

/// Convert world position to chunk coordinates
fn world_to_chunk(x: f32, z: f32) -> (i32, i32) {
    (
        (x / CHUNK_SIZE).floor() as i32,
        (z / CHUNK_SIZE).floor() as i32,
    )
}

/// Convert chunk coordinates to world position (center of chunk)
fn chunk_to_world(chunk_x: i32, chunk_z: i32) -> (f32, f32) {
    (
        chunk_x as f32 * CHUNK_SIZE + CHUNK_SIZE / 2.0,
        chunk_z as f32 * CHUNK_SIZE + CHUNK_SIZE / 2.0,
    )
}

/// Main terrain update system
#[allow(clippy::too_many_arguments)]
pub fn update_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    terrain_chunks: Query<(Entity, &Terrain, &Handle<Mesh>, Option<&TerrainCollider>, Option<&FrustumHidden>)>,
    mut grid_query: Query<&mut TerrainGrid>,
    player: Query<&Transform, With<player::Player>>,
    camera: Query<(&Frustum, &GlobalTransform), With<Camera3d>>,
) {
    let Ok(player_trans) = player.get_single() else { return };
    let player_pos = player_trans.translation;
    let (player_chunk_x, player_chunk_z) = world_to_chunk(player_pos.x, player_pos.z);

    // Get camera frustum for visibility checks
    let camera_frustum = camera.get_single().ok();

    if terrain_chunks.is_empty() {
        // Initial spawn - create terrain grid and spawn chunks
        let mut terrain_grid = TerrainGrid(HashMap::new());

        for dx in -CHUNKS_RADIUS..=CHUNKS_RADIUS {
            for dz in -CHUNKS_RADIUS..=CHUNKS_RADIUS {
                let chunk_x = player_chunk_x + dx;
                let chunk_z = player_chunk_z + dz;
                let (world_x, world_z) = chunk_to_world(chunk_x, chunk_z);

                // Calculate distance for LOD
                let chunk_distance = dx.abs().max(dz.abs());
                let lod_level = get_lod_level(chunk_distance);

                // Check frustum visibility
                let chunk_center = Vec3::new(world_x, player_pos.y, world_z);
                let in_frustum = camera_frustum
                    .map(|(frustum, _)| is_chunk_in_frustum(frustum, chunk_center))
                    .unwrap_or(true);

                if in_frustum {
                    // Spawn chunks with colliders synchronously (player needs them immediately)
                    // Spawn distant chunks asynchronously
                    if chunk_distance <= COLLIDER_DISTANCE {
                        terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::visible(lod_level));
                        spawn_terrain_chunk(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            chunk_x,
                            chunk_z,
                            lod_level,
                            true,
                        );
                    } else {
                        terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::pending());
                        spawn_terrain_chunk_async(
                            &mut commands,
                            chunk_x,
                            chunk_z,
                            lod_level,
                            false,
                        );
                    }
                } else {
                    // Mark as hidden in the grid - no entity spawned yet
                    terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::hidden(lod_level));
                }
            }
        }

        commands.spawn(terrain_grid);
        spawn_water_plane(&mut commands, &mut meshes, &mut materials, &asset_server);
    } else {
        // Main update logic
        let Ok(mut terrain_grid) = grid_query.get_single_mut() else { return };

        // Update grid state for completed async tasks (Pending -> Visible)
        for (_, terrain, _, _, _) in terrain_chunks.iter() {
            if let Some(state) = terrain_grid.0.get(&(terrain.chunk_x, terrain.chunk_z)) {
                if state.render_state == RenderState::Pending {
                    terrain_grid.0.insert((terrain.chunk_x, terrain.chunk_z), TerrainChunkState::visible(terrain.lod_level));
                }
            }
        }

        // Track chunks to despawn (too far), hide (frustum culled), or show (was hidden, now visible)
        let mut chunks_to_despawn: Vec<Entity> = Vec::new();
        let mut chunks_to_hide: Vec<(Entity, i32, i32, u32)> = Vec::new();
        let mut chunks_to_show: Vec<Entity> = Vec::new();
        let mut chunks_to_update: Vec<(Entity, i32, i32, u32, Handle<Mesh>)> = Vec::new();
        // Track chunks that need collider added/removed (without LOD change)
        let mut chunks_need_collider: Vec<(Entity, Handle<Mesh>)> = Vec::new();
        let mut chunks_remove_collider: Vec<Entity> = Vec::new();

        // Check existing chunks for despawn/hide/show/LOD update/collider update
        for (entity, terrain, mesh_handle, has_collider, is_hidden) in terrain_chunks.iter() {
            let dx = terrain.chunk_x - player_chunk_x;
            let dz = terrain.chunk_z - player_chunk_z;
            let chunk_distance = dx.abs().max(dz.abs());

            // Despawn if too far (beyond render distance)
            if chunk_distance > CHUNKS_RADIUS {
                terrain_grid.0.remove(&(terrain.chunk_x, terrain.chunk_z));
                chunks_to_despawn.push(entity);
                continue;
            }

            // Check frustum visibility
            let (world_x, world_z) = chunk_to_world(terrain.chunk_x, terrain.chunk_z);
            let chunk_center = Vec3::new(world_x, player_pos.y, world_z);
            let in_frustum = camera_frustum
                .map(|(frustum, _)| is_chunk_in_frustum(frustum, chunk_center))
                .unwrap_or(true);

            if !in_frustum {
                // Hide instead of despawn - only if not already hidden
                if is_hidden.is_none() {
                    chunks_to_hide.push((entity, terrain.chunk_x, terrain.chunk_z, terrain.lod_level));
                }
                continue;
            }

            // If was hidden but now in frustum, show it
            if is_hidden.is_some() {
                chunks_to_show.push(entity);
                terrain_grid.0.insert((terrain.chunk_x, terrain.chunk_z), TerrainChunkState::visible(terrain.lod_level));
            }

            // Check if LOD needs to change
            let new_lod = get_lod_level(chunk_distance);
            if new_lod != terrain.lod_level {
                chunks_to_update.push((entity, terrain.chunk_x, terrain.chunk_z, new_lod, mesh_handle.clone()));
            } else {
                // LOD unchanged, but check if collider state needs to change
                let needs_collider = chunk_distance <= COLLIDER_DISTANCE;
                if needs_collider && has_collider.is_none() {
                    chunks_need_collider.push((entity, mesh_handle.clone()));
                } else if !needs_collider && has_collider.is_some() {
                    chunks_remove_collider.push(entity);
                }
            }
        }

        // Despawn chunks that are too far
        for entity in chunks_to_despawn {
            commands.entity(entity).despawn_recursive();
        }

        // Hide chunks outside frustum (keep entity, just hide it)
        for (entity, chunk_x, chunk_z, lod_level) in chunks_to_hide {
            terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::hidden(lod_level));
            commands.entity(entity)
                .insert(FrustumHidden)
                .insert(Visibility::Hidden);
        }

        // Show chunks that were hidden but are now visible
        for entity in chunks_to_show {
            commands.entity(entity)
                .remove::<FrustumHidden>()
                .insert(Visibility::Visible);
        }

        // Update LOD for existing chunks
        for (entity, chunk_x, chunk_z, new_lod, mesh_handle) in chunks_to_update {
            let (world_x, world_z) = chunk_to_world(chunk_x, chunk_z);
            let new_mesh = generate_terrain_mesh(world_x, world_z, CHUNK_SIZE, new_lod);

            if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                *mesh = new_mesh.clone();
            }

            let dx = chunk_x - player_chunk_x;
            let dz = chunk_z - player_chunk_z;
            let chunk_distance = dx.abs().max(dz.abs());

            // Update collider if close enough
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(Terrain {
                chunk_x,
                chunk_z,
                lod_level: new_lod,
            });

            if chunk_distance <= COLLIDER_DISTANCE {
                let collider_shape = ComputedColliderShape::TriMesh;
                if let Some(collider) = Collider::from_bevy_mesh(&new_mesh, &collider_shape) {
                    entity_commands.insert(collider);
                    entity_commands.insert(TerrainCollider);
                }
            } else {
                entity_commands.remove::<Collider>();
                entity_commands.remove::<TerrainCollider>();
            }

            terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::visible(new_lod));
        }

        // Add colliders to chunks that now need them (player approached)
        for (entity, mesh_handle) in chunks_need_collider {
            if let Some(mesh) = meshes.get(&mesh_handle) {
                let collider_shape = ComputedColliderShape::TriMesh;
                if let Some(collider) = Collider::from_bevy_mesh(mesh, &collider_shape) {
                    commands.entity(entity).insert(collider).insert(TerrainCollider);
                }
            }
        }

        // Remove colliders from chunks that no longer need them (player moved away)
        for entity in chunks_remove_collider {
            commands.entity(entity).remove::<Collider>().remove::<TerrainCollider>();
        }

        // Collect chunks that need to be spawned
        let mut chunks_to_spawn: Vec<(i32, i32, i32, u32, bool)> = Vec::new(); // (chunk_x, chunk_z, distance, lod, needs_collider)

        for dx in -CHUNKS_RADIUS..=CHUNKS_RADIUS {
            for dz in -CHUNKS_RADIUS..=CHUNKS_RADIUS {
                let chunk_x = player_chunk_x + dx;
                let chunk_z = player_chunk_z + dz;

                // Check if chunk already exists or is being handled
                let existing_state = terrain_grid.0.get(&(chunk_x, chunk_z));

                let (world_x, world_z) = chunk_to_world(chunk_x, chunk_z);
                let chunk_center = Vec3::new(world_x, player_pos.y, world_z);
                let in_frustum = camera_frustum
                    .map(|(frustum, _)| is_chunk_in_frustum(frustum, chunk_center))
                    .unwrap_or(true);

                match existing_state {
                    None => {
                        // New chunk - queue for spawn if in frustum
                        if in_frustum {
                            let chunk_distance = dx.abs().max(dz.abs());
                            let lod_level = get_lod_level(chunk_distance);
                            let needs_collider = chunk_distance <= COLLIDER_DISTANCE;
                            chunks_to_spawn.push((chunk_x, chunk_z, chunk_distance, lod_level, needs_collider));
                        } else {
                            // Mark as hidden in grid - no entity exists yet
                            let chunk_distance = dx.abs().max(dz.abs());
                            let lod_level = get_lod_level(chunk_distance);
                            terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::hidden(lod_level));
                        }
                    }
                    Some(state) if state.render_state == RenderState::Hidden => {
                        // Hidden chunk - if in frustum and no entity exists, spawn it
                        // (Entities with FrustumHidden are handled above in the show logic)
                        if in_frustum {
                            let chunk_distance = dx.abs().max(dz.abs());
                            let lod_level = get_lod_level(chunk_distance);
                            let needs_collider = chunk_distance <= COLLIDER_DISTANCE;
                            chunks_to_spawn.push((chunk_x, chunk_z, chunk_distance, lod_level, needs_collider));
                        }
                    }
                    Some(state) if state.render_state == RenderState::Pending => {
                        // Already being generated - nothing to do
                    }
                    Some(state) if state.render_state == RenderState::Visible => {
                        // Already visible - nothing to do
                    }
                    _ => {}
                }
            }
        }

        // Sort chunks by distance (spawn closer chunks first, prioritize chunks needing colliders)
        chunks_to_spawn.sort_by_key(|(_, _, distance, _, needs_collider)| {
            // Chunks needing colliders get highest priority (negative distance)
            if *needs_collider { -100 + *distance } else { *distance }
        });

        // Spawn chunks with rate limiting
        let mut async_spawns_this_frame = 0;
        for (chunk_x, chunk_z, _distance, lod_level, needs_collider) in chunks_to_spawn {
            if needs_collider {
                // Spawn synchronously - player might need this for physics
                terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::visible(lod_level));
                spawn_terrain_chunk(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    chunk_x,
                    chunk_z,
                    lod_level,
                    true,
                );
            } else if async_spawns_this_frame < MAX_CHUNKS_PER_FRAME {
                // Spawn asynchronously with rate limiting
                terrain_grid.0.insert((chunk_x, chunk_z), TerrainChunkState::pending());
                spawn_terrain_chunk_async(
                    &mut commands,
                    chunk_x,
                    chunk_z,
                    lod_level,
                    false,
                );
                async_spawns_this_frame += 1;
            }
            // Chunks beyond the rate limit will be picked up next frame
        }

        // Clean up grid entries for chunks that are too far
        terrain_grid.0.retain(|(cx, cz), _| {
            let dx = (*cx - player_chunk_x).abs();
            let dz = (*cz - player_chunk_z).abs();
            dx <= CHUNKS_RADIUS + 1 && dz <= CHUNKS_RADIUS + 1
        });
    }
}

fn get_terrain_color(y: f32) -> [f32;4] {
    if y < HEIGHT_SAND { COLOR_SAND }
    else if y > HEIGHT_PEAKS { COLOR_PEAKS }
    else if y < HEIGHT_TEMPERATE_START {
        terrain_color_gradient(
            (y-HEIGHT_SAND)/(HEIGHT_TEMPERATE_START-HEIGHT_SAND),
            COLOR_SAND,
            COLOR_TEMPERATE
        )
    } else if y < HEIGHT_TEMPERATE_END {
        COLOR_TEMPERATE
    } else {
        terrain_color_gradient(
            (y-HEIGHT_TEMPERATE_END)/(HEIGHT_PEAKS-HEIGHT_TEMPERATE_END),
            COLOR_TEMPERATE,
            COLOR_PEAKS
        )
    }
}

fn terrain_color_gradient(ratio: f32, rgba1: [f32; 4], rgba2: [f32; 4]) -> [f32;4] {
    let [r1, g1, b1, a1] = rgba1;
    let [r2, g2, b2, a2] = rgba2;

    [
        r1 + (r2-r1)*(ratio),
        g1 + (g2-g1)*(ratio),
        b1 + (b2-b1)*(ratio),
        a1 + (a2-a1)*(ratio)
    ]
}

/// Spawn a terrain chunk at the given chunk coordinates
fn spawn_terrain_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_x: i32,
    chunk_z: i32,
    subdivisions: u32,
    with_collider: bool,
) -> Entity {
    let (world_x, world_z) = chunk_to_world(chunk_x, chunk_z);
    let mesh = generate_terrain_mesh(world_x, world_z, CHUNK_SIZE, subdivisions);

    let terrain_material = StandardMaterial {
        alpha_mode: AlphaMode::Opaque,
        double_sided: true,
        perceptual_roughness: 1.0,
        reflectance: 0.4,
        cull_mode: Some(Face::Back),
        flip_normal_map_y: true,
        ..default()
    };

    let mut entity_commands = commands.spawn(PbrBundle {
        mesh: meshes.add(mesh.clone()),
        material: materials.add(terrain_material),
        transform: Transform::from_xyz(world_x, 0., world_z),
        ..default()
    });

    entity_commands.insert(Terrain {
        chunk_x,
        chunk_z,
        lod_level: subdivisions,
    });

    if with_collider {
        let collider_shape = ComputedColliderShape::TriMesh;
        if let Some(collider) = Collider::from_bevy_mesh(&mesh, &collider_shape) {
            entity_commands.insert(collider);
            entity_commands.insert(TerrainCollider);
        }
    }

    entity_commands.id()
}

/// Spawn a terrain chunk asynchronously to avoid blocking the main thread
fn spawn_terrain_chunk_async(
    commands: &mut Commands,
    chunk_x: i32,
    chunk_z: i32,
    subdivisions: u32,
    with_collider: bool,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let (world_x, world_z) = chunk_to_world(chunk_x, chunk_z);
    let transform = Transform::from_xyz(world_x, 0., world_z);

    let task_entity = commands.spawn_empty().id();

    let task = thread_pool.spawn(async move {
        let mut command_queue = CommandQueue::default();

        // Generate mesh on background thread
        let mesh = generate_terrain_mesh(world_x, world_z, CHUNK_SIZE, subdivisions);
        let mesh_for_collider = if with_collider { Some(mesh.clone()) } else { None };

        command_queue.push(move |world: &mut World| {
            let (mesh_handle, material_handle) = {
                let mut system_state = SystemState::<(
                    ResMut<Assets<Mesh>>,
                    ResMut<Assets<StandardMaterial>>,
                )>::new(world);
                let (mut meshes, mut materials) = system_state.get_mut(world);

                let terrain_material = StandardMaterial {
                    alpha_mode: AlphaMode::Opaque,
                    double_sided: true,
                    perceptual_roughness: 1.0,
                    reflectance: 0.4,
                    cull_mode: Some(Face::Back),
                    flip_normal_map_y: true,
                    ..default()
                };

                (meshes.add(mesh), materials.add(terrain_material))
            };

            let mut entity = world.entity_mut(task_entity);
            entity
                .insert(PbrBundle {
                    mesh: mesh_handle,
                    material: material_handle,
                    transform,
                    ..default()
                })
                .insert(Terrain {
                    chunk_x,
                    chunk_z,
                    lod_level: subdivisions,
                })
                .remove::<GenTerrainTask>();

            // Add collider if needed
            if let Some(collider_mesh) = mesh_for_collider {
                let collider_shape = ComputedColliderShape::TriMesh;
                if let Some(collider) = Collider::from_bevy_mesh(&collider_mesh, &collider_shape) {
                    entity.insert(collider);
                    entity.insert(TerrainCollider);
                }
            }
        });

        command_queue
    });

    commands.entity(task_entity).insert(GenTerrainTask(task));
}

/// Handle completed async terrain generation tasks
fn handle_terrain_tasks(mut commands: Commands, mut terrain_tasks: Query<&mut GenTerrainTask>) {
    for mut task in &mut terrain_tasks {
        if let Some(mut commands_queue) = block_on(poll_once(&mut task.0)) {
            commands.append(&mut commands_queue);
        }
    }
}

#[derive(Component)]
struct Water;

/// Generate a simple plane mesh (used for water)
fn generate_simple_plane(size: f32, subdivisions: u32) -> Mesh {
    let asset_usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, asset_usage);

    let segments = subdivisions + 1;
    let half_size = size / 2.0;
    let segment_size = size / segments as f32;
    let vertex_count = ((segments + 1) * (segments + 1)) as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);

    for z in 0..=segments {
        for x in 0..=segments {
            let px = -half_size + x as f32 * segment_size;
            let pz = -half_size + z as f32 * segment_size;
            positions.push([px, 0.0, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / segments as f32, z as f32 / segments as f32]);
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity((segments * segments * 6) as usize);
    for z in 0..segments {
        for x in 0..segments {
            let i = z * (segments + 1) + x;
            indices.push(i);
            indices.push(i + segments + 1);
            indices.push(i + 1);
            indices.push(i + 1);
            indices.push(i + segments + 1);
            indices.push(i + segments + 2);
        }
    }

    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    mesh
}

fn spawn_water_plane(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    let water_size = CHUNK_SIZE * (CHUNKS_RADIUS as f32 * 2.0 + 1.0);
    let mut water_mesh = generate_simple_plane(water_size, 1);

    let pos_attr = water_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    let VertexAttributeValues::Float32x3(pos_attr) = pos_attr else {
        panic!("Unexpected vertex format, expected Float32x3");
    };

    let water_uvs: Vec<[f32; 2]> = pos_attr.iter().map(|[x,_y,z]| { [x / WATER_TEXTURE_SCALE, z / WATER_TEXTURE_SCALE]}).collect();

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

    commands.spawn(PbrBundle {
        mesh: meshes.add(water_mesh),
        material: materials.add(StandardMaterial {
            base_color: Color::rgba(0.,54./256.,78./256., 236./256.),
            perceptual_roughness: 0.7,
            metallic: 0.2,
            reflectance: 0.45,
            diffuse_transmission: 0.0,
            specular_transmission:0.3,
            normal_map_texture: Some(normal_handle.clone()),
            flip_normal_map_y: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, WATER_LEVEL, 0.0),
        ..default()
    }).insert(Water);
}

/// Generate a terrain mesh with custom mesh generation (replaces deprecated Plane)
/// Creates a subdivided plane with height sampling from Perlin noise
fn generate_terrain_mesh(center_x: f32, center_z: f32, size: f32, subdivisions: u32) -> Mesh {
    let asset_usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, asset_usage);

    let height_map = perlin::terrain_perlin();
    let half_size = size / 2.0;
    let segment_size = size / subdivisions as f32;
    let vertex_count = ((subdivisions + 1) * (subdivisions + 1)) as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);
    let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(vertex_count);

    // Generate vertices with height from Perlin noise
    for z in 0..=subdivisions {
        for x in 0..=subdivisions {
            // Local position relative to chunk center
            let local_x = -half_size + x as f32 * segment_size;
            let local_z = -half_size + z as f32 * segment_size;

            // World position for height sampling
            let world_x = center_x + local_x;
            let world_z = center_z + local_z;

            let height = sample_terrain_height(&height_map, world_x, world_z);

            positions.push([local_x, height, local_z]);
            normals.push([0.0, 1.0, 0.0]); // Will be recalculated below
            uvs.push([local_x / (TILE_WIDTH as f32 * TEXTURE_SCALE), local_z / (TILE_WIDTH as f32 * TEXTURE_SCALE)]);
            vertex_colors.push(get_terrain_color(height));
        }
    }

    // Generate indices
    let mut indices: Vec<u32> = Vec::with_capacity((subdivisions * subdivisions * 6) as usize);
    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let i = z * (subdivisions + 1) + x;
            // First triangle
            indices.push(i);
            indices.push(i + subdivisions + 1);
            indices.push(i + 1);
            // Second triangle
            indices.push(i + 1);
            indices.push(i + subdivisions + 1);
            indices.push(i + subdivisions + 2);
        }
    }

    // Calculate proper normals based on terrain geometry
    let row_size = (subdivisions + 1) as usize;
    for z in 0..=subdivisions as usize {
        for x in 0..=subdivisions as usize {
            let idx = z * row_size + x;
            let pos = positions[idx];

            // Get neighboring heights for normal calculation
            let left = if x > 0 { positions[idx - 1][1] } else { pos[1] };
            let right = if x < subdivisions as usize { positions[idx + 1][1] } else { pos[1] };
            let up = if z > 0 { positions[idx - row_size][1] } else { pos[1] };
            let down = if z < subdivisions as usize { positions[idx + row_size][1] } else { pos[1] };

            // Calculate normal from height differences
            let dx = (right - left) / (2.0 * segment_size);
            let dz = (down - up) / (2.0 * segment_size);
            let normal = Vec3::new(-dx, 1.0, -dz).normalize();
            normals[idx] = normal.to_array();
        }
    }

    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);

    let _ = mesh.generate_tangents();

    mesh
}

fn update_water(
    _commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
    mut water: Query<(Entity,&Handle<Mesh>), With<Water> >,
) {
    let Ok((_water_ent, water_mesh_handle)) = water.get_single_mut() else {
        return
    };
    let water_mesh = meshes.get_mut(water_mesh_handle).unwrap();
    let water_uvs = water_mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap();
    let VertexAttributeValues::Float32x2(uv_attr) = water_uvs else {
        panic!("Unexpected vertex format, expected Float32x3");
    };
    for [x,y] in uv_attr.iter_mut() {
        *x += WATER_SCROLL_SPEED;
        *y += WATER_SCROLL_SPEED;
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_terrain, handle_terrain_tasks, update_water));
    }
}