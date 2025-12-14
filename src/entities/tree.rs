use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::math::Vec3A;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexBufferLayout, VertexFormat};
use bevy::render::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayout};
use bevy::render::primitives::Aabb;
use bevy::utils::HashMap;
use bevy::ecs::system::{CommandQueue, SystemState};
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use futures_lite::future::poll_once;
use rand::{thread_rng, Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::util::perlin::sample_terrain_height;
use crate::util::perlin;
use crate::util::render_state::RenderState;
use crate::entities::terrain::{HEIGHT_TEMPERATE_START, HEIGHT_TEMPERATE_END};
use crate::entities::grass::{GRASS_BASE_COLOR_2, GRASS_SECOND_COLOR};
use super::player::ContainsPlayer;

const ATTRIBUTE_BASE_Y: MeshVertexAttribute = MeshVertexAttribute::new("BaseY", 988540917, VertexFormat::Float32);
const ATTRIBUTE_STARTING_POSITION: MeshVertexAttribute = MeshVertexAttribute::new("StartingPosition", 988540916, VertexFormat::Float32x3);
const ATTRIBUTE_WORLD_POSITION: MeshVertexAttribute = MeshVertexAttribute::new("WorldPosition", 988540915, VertexFormat::Float32x3);

// Tree tile constants
const TREE_TILE_SIZE: f32 = 64.0;
const TREES_PER_TILE: u32 = 12; // Trees per tile (sparse compared to grass)
const GRID_SIZE_HALF: i32 = 12; // View distance in tiles
const DESPAWN_DISTANCE: f32 = (GRID_SIZE_HALF + 1) as f32 * TREE_TILE_SIZE + GRID_SIZE_HALF as f32;

// LOD distances (in tiles from player)
const LOD_HIGH_DISTANCE: i32 = 3; // High detail within 3 tiles, billboard beyond

// Tree geometry constants
const TRUNK_RADIUS: f32 = 0.3;
const TRUNK_HEIGHT: f32 = 10.0;
const TRUNK_SEGMENTS: u32 = 8;
const TRUNK_COLOR_BASE: [f32; 4] = [0.30, 0.18, 0.08, 1.0];
// TRUNK_COLOR_TOP uses GRASS_BASE_COLOR_2 - set in generate_cylinder_trunk

// Foliage base color close to trunk, tip gradients to grass color
const FOLIAGE_COLOR_BASE: [f32; 4] = [0.12, 0.10, 0.04, 1.0]; // Greenish-brown near trunk

const FOLIAGE_TIERS: u32 = 48;
const BLADES_PER_TIER: u32 = 16;
const FOLIAGE_START_HEIGHT: f32 = 1.5;
const TREE_TOP_HEIGHT: f32 = 14.0;
const BASE_TIER_RADIUS: f32 = 3.5;
const TOP_TIER_RADIUS: f32 = 0.2;
const BLADE_LENGTH_BASE: f32 = 2.8;
const BLADE_LENGTH_TOP: f32 = 0.5;
const BLADE_WIDTH: f32 = 0.35;

const UPWARD_ANGLE_BASE: f32 = 0.15;
const UPWARD_ANGLE_TOP: f32 = 1.2;

// Billboard (low LOD) constants
const BILLBOARD_TRUNK_WIDTH: f32 = 0.8;
const BILLBOARD_TRUNK_HEIGHT: f32 = 4.0;
const BILLBOARD_FOLIAGE_WIDTH: f32 = 6.0;
const BILLBOARD_FOLIAGE_HEIGHT: f32 = 10.0;
const BILLBOARD_TRUNK_COLOR: [f32; 4] = [0.35, 0.22, 0.10, 1.0];
const BILLBOARD_FOLIAGE_COLOR: [f32; 4] = [0.02, 0.25, 0.06, 1.0];

#[derive(Component)]
pub struct Tree;

#[derive(Component)]
pub struct TreeTile {
    pub lod_level: u32, // 0 = high detail, 1 = billboard
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TreeTileState {
    pub render_state: RenderState,
    pub lod_level: u32,
}

impl TreeTileState {
    pub fn visible(lod_level: u32) -> Self {
        Self { render_state: RenderState::Visible, lod_level }
    }

    pub fn pending() -> Self {
        Self { render_state: RenderState::Pending, lod_level: 0 }
    }
}

#[derive(Component)]
struct TreeGrid(HashMap<(i32, i32), TreeTileState>);

#[derive(Component)]
struct GenTreeTask(Task<CommandQueue>);

/// Create an Aabb for a tree tile (centered at local origin)
/// Tree mesh vertices are at actual terrain heights (not relative to transform)
fn tree_tile_aabb() -> Aabb {
    let half_size = TREE_TILE_SIZE / 2.0;
    // Trees exist in temperate zone (210-800) plus tree height above terrain
    let min_height = HEIGHT_TEMPERATE_START - 10.0;
    let max_height = HEIGHT_TEMPERATE_END + TREE_TOP_HEIGHT + 5.0;
    let center_y = (min_height + max_height) / 2.0;
    let half_height = (max_height - min_height) / 2.0;
    Aabb {
        center: Vec3A::new(0.0, center_y, 0.0),
        half_extents: Vec3A::new(half_size, half_height, half_size),
    }
}

fn get_lod_level(tile_distance: i32) -> u32 {
    if tile_distance <= LOD_HIGH_DISTANCE {
        0 // High detail
    } else {
        1 // Billboard
    }
}

/// Generate high-detail tree mesh for a single tree
fn generate_tree_mesh_at(local_x: f32, y: f32, local_z: f32) -> (Vec<Vec3>, Vec<u32>, Vec<[f32; 4]>) {
    let mut all_verts: Vec<Vec3> = vec![];
    let mut all_indices: Vec<u32> = vec![];
    let mut all_colors: Vec<[f32; 4]> = vec![];
    let mut vertex_count: u32 = 0;

    // Generate cylinder trunk
    let (trunk_verts, trunk_indices, trunk_colors) = generate_cylinder_trunk(local_x, y, local_z, vertex_count);
    vertex_count += trunk_verts.len() as u32;
    all_verts.extend(trunk_verts);
    all_indices.extend(trunk_indices);
    all_colors.extend(trunk_colors);

    // Generate foliage tiers
    for tier in 0..FOLIAGE_TIERS {
        let tier_ratio = tier as f32 / (FOLIAGE_TIERS - 1) as f32;
        let tier_height = y + FOLIAGE_START_HEIGHT + (TREE_TOP_HEIGHT - FOLIAGE_START_HEIGHT) * tier_ratio;
        let tier_radius = BASE_TIER_RADIUS * (1.0 - tier_ratio) + TOP_TIER_RADIUS * tier_ratio;
        let blade_length = BLADE_LENGTH_BASE * (1.0 - tier_ratio) + BLADE_LENGTH_TOP * tier_ratio;
        let upward_angle = UPWARD_ANGLE_BASE * (1.0 - tier_ratio) + UPWARD_ANGLE_TOP * tier_ratio;

        let (tier_verts, tier_indices, tier_colors) = generate_foliage_tier(
            local_x, tier_height, local_z,
            tier_radius,
            blade_length,
            upward_angle,
            tier_ratio,
            vertex_count,
        );
        vertex_count += tier_verts.len() as u32;
        all_verts.extend(tier_verts);
        all_indices.extend(tier_indices);
        all_colors.extend(tier_colors);
    }

    (all_verts, all_indices, all_colors)
}

/// Generate billboard (low LOD) tree mesh for a single tree
fn generate_billboard_tree_at(local_x: f32, y: f32, local_z: f32, base_index: u32) -> (Vec<Vec3>, Vec<u32>, Vec<[f32; 4]>) {
    let mut verts = vec![];
    let mut indices = vec![];
    let mut colors = vec![];

    let half_trunk_w = BILLBOARD_TRUNK_WIDTH / 2.0;
    let half_foliage_w = BILLBOARD_FOLIAGE_WIDTH / 2.0;

    // Trunk quad (two triangles)
    let t0 = Vec3::new(local_x - half_trunk_w, y, local_z);
    let t1 = Vec3::new(local_x + half_trunk_w, y, local_z);
    let t2 = Vec3::new(local_x + half_trunk_w, y + BILLBOARD_TRUNK_HEIGHT, local_z);
    let t3 = Vec3::new(local_x - half_trunk_w, y + BILLBOARD_TRUNK_HEIGHT, local_z);

    let idx = base_index;
    verts.extend([t0, t1, t2, t3]);
    colors.extend([BILLBOARD_TRUNK_COLOR; 4]);
    indices.extend([idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

    // Foliage triangle
    let foliage_base_y = y + BILLBOARD_TRUNK_HEIGHT * 0.5;
    let f0 = Vec3::new(local_x - half_foliage_w, foliage_base_y, local_z);
    let f1 = Vec3::new(local_x + half_foliage_w, foliage_base_y, local_z);
    let f2 = Vec3::new(local_x, foliage_base_y + BILLBOARD_FOLIAGE_HEIGHT, local_z);

    let fidx = base_index + 4;
    verts.extend([f0, f1, f2]);
    colors.extend([GRASS_SECOND_COLOR; 3]);
    indices.extend([fidx, fidx + 1, fidx + 2]);

    (verts, indices, colors)
}

fn generate_cylinder_trunk(x: f32, y: f32, z: f32, base_index: u32) -> (Vec<Vec3>, Vec<u32>, Vec<[f32; 4]>) {
    let mut verts = vec![];
    let mut indices = vec![];
    let mut colors = vec![];

    for i in 0..TRUNK_SEGMENTS {
        let angle = (i as f32 / TRUNK_SEGMENTS as f32) * std::f32::consts::TAU;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let bottom = Vec3::new(x + cos_a * TRUNK_RADIUS, y, z + sin_a * TRUNK_RADIUS);
        let top = Vec3::new(x + cos_a * TRUNK_RADIUS, y + TRUNK_HEIGHT, z + sin_a * TRUNK_RADIUS);

        verts.push(bottom);
        verts.push(top);
        colors.push(TRUNK_COLOR_BASE);
        colors.push(GRASS_BASE_COLOR_2);
    }

    for i in 0..TRUNK_SEGMENTS {
        let next = (i + 1) % TRUNK_SEGMENTS;
        let b0 = base_index + i * 2;
        let t0 = base_index + i * 2 + 1;
        let b1 = base_index + next * 2;
        let t1 = base_index + next * 2 + 1;

        indices.extend([b0, b1, t1, b0, t1, t0]);
    }

    (verts, indices, colors)
}

fn generate_foliage_tier(
    center_x: f32, center_y: f32, center_z: f32,
    radius: f32,
    blade_length: f32,
    upward_angle: f32,
    tier_ratio: f32,
    base_index: u32,
) -> (Vec<Vec3>, Vec<u32>, Vec<[f32; 4]>) {
    let mut verts = vec![];
    let mut indices = vec![];
    let mut colors = vec![];
    let mut rng = thread_rng();

    for i in 0..BLADES_PER_TIER {
        let base_angle = (i as f32 / BLADES_PER_TIER as f32) * std::f32::consts::TAU;
        let angle = base_angle + rng.gen_range(-0.15..0.15);

        let blade_base_x = center_x + angle.cos() * (TRUNK_RADIUS * 0.5);
        let blade_base_z = center_z + angle.sin() * (TRUNK_RADIUS * 0.5);

        let horizontal_dist = blade_length * upward_angle.cos();
        let vertical_dist = blade_length * upward_angle.sin();

        let tip_x = center_x + angle.cos() * (radius + horizontal_dist);
        let tip_z = center_z + angle.sin() * (radius + horizontal_dist);
        let tip_y = center_y + vertical_dist;

        let perp_angle = angle + std::f32::consts::FRAC_PI_2;
        let half_width = BLADE_WIDTH / 2.0;

        let v0 = Vec3::new(
            blade_base_x + perp_angle.cos() * half_width,
            center_y,
            blade_base_z + perp_angle.sin() * half_width,
        );
        let v1 = Vec3::new(
            blade_base_x - perp_angle.cos() * half_width,
            center_y,
            blade_base_z - perp_angle.sin() * half_width,
        );
        let v2 = Vec3::new(tip_x, tip_y, tip_z);

        let idx = base_index + verts.len() as u32;
        verts.extend([v0, v1, v2]);
        indices.extend([idx, idx + 1, idx + 2]);

        let base_color = color_lerp(FOLIAGE_COLOR_BASE, GRASS_BASE_COLOR_2, tier_ratio * 0.3);
        let tip_color = color_lerp(FOLIAGE_COLOR_BASE, GRASS_BASE_COLOR_2, tier_ratio * 0.3 + 0.5);
        colors.extend([base_color, base_color, tip_color]);
    }

    (verts, indices, colors)
}

fn color_lerp(c1: [f32; 4], c2: [f32; 4], t: f32) -> [f32; 4] {
    [
        c1[0] + (c2[0] - c1[0]) * t,
        c1[1] + (c2[1] - c1[1]) * t,
        c1[2] + (c2[2] - c1[2]) * t,
        c1[3] + (c2[3] - c1[3]) * t,
    ]
}

/// Generate a tile of trees (either high or low LOD)
fn generate_tree_tile_mesh(tile_x: f32, tile_z: f32, lod_level: u32) -> Mesh {
    let asset_usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, asset_usage);

    let terrain_perlin = perlin::terrain_perlin();

    let mut all_verts: Vec<Vec3> = vec![];
    let mut bases: Vec<f32> = Vec::new();
    let mut tree_offsets = Vec::new();
    let mut all_indices: Vec<u32> = vec![];
    let mut all_colors: Vec<[f32; 4]> = vec![];
    let mut vertex_count: u32 = 0;

    // Use deterministic RNG based on tile position for consistent tree placement
    let seed = ((tile_x as i32).wrapping_mul(73856093) ^ (tile_z as i32).wrapping_mul(19349663)) as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    let half_tile = TREE_TILE_SIZE / 2.0;

    for _ in 0..TREES_PER_TILE {
        let local_x = rng.gen_range(-half_tile..half_tile);
        let local_z = rng.gen_range(-half_tile..half_tile);

        let world_x = tile_x + local_x;
        let world_z = tile_z + local_z;
        let y = sample_terrain_height(&terrain_perlin, world_x, world_z);

        // Only place trees in temperate zone
        if y < HEIGHT_TEMPERATE_START || y > HEIGHT_TEMPERATE_END {
            continue;
        }

        if lod_level == 0 {
            // High detail tree
            let (mut verts, mut indices, mut colors) = generate_tree_mesh_at(local_x, y, local_z);
            // Offset indices
            for idx in &mut indices {
                *idx += vertex_count;
            }
            vertex_count += verts.len() as u32;
            for _ in 0..verts.len() {
                bases.push(y);
                tree_offsets.push([world_x, y, world_z]);
            }
            all_verts.append(&mut verts);
            all_indices.append(&mut indices);
            all_colors.append(&mut colors);
        } else {
            // Billboard tree
            let (mut verts, mut indices, mut colors) = generate_billboard_tree_at(local_x, y, local_z, vertex_count);
            vertex_count += verts.len() as u32;
            for _ in 0..verts.len() {
                bases.push(y);
                tree_offsets.push([world_x, y, world_z]);
            }
            all_verts.append(&mut verts);
            all_indices.append(&mut indices);
            all_colors.append(&mut colors);
        }
    }

    let positions: Vec<[f32; 3]> = all_verts.iter().map(|v| v.to_array()).collect();
    let normals: Vec<[f32; 3]> = all_verts.iter().map(|_| [0.0, 1.0, 0.0]).collect();

    mesh.insert_indices(Indices::U32(all_indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, all_colors);

    mesh.insert_attribute(ATTRIBUTE_BASE_Y, bases);
    mesh.insert_attribute(ATTRIBUTE_STARTING_POSITION, positions);
    mesh.insert_attribute(ATTRIBUTE_WORLD_POSITION, tree_offsets);

    mesh
}

fn tree_material() -> ExtendedMaterial<StandardMaterial, TreeMaterialExtension> {
    ExtendedMaterial { base: StandardMaterial {
        base_color: Color::WHITE,
        double_sided: false,
        perceptual_roughness: 1.0,
        reflectance: 0.3,
        cull_mode: None,
        opaque_render_method: bevy::pbr::OpaqueRendererMethod::Forward,
        unlit: false,
        ..default()
    },
    extension: TreeMaterialExtension {  }
}
}

fn spawn_tree_tile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, TreeMaterialExtension>>>,
    tile_x: f32,
    tile_z: f32,
    lod_level: u32,
) -> Entity {
    let mesh = generate_tree_tile_mesh(tile_x, tile_z, lod_level);

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(mesh),
        material: materials.add(tree_material()),
        transform: Transform::from_xyz(tile_x, 0.0, tile_z),
        ..default()
    })
    .insert(Tree)
    .insert(TreeTile { lod_level })
    .insert(ContainsPlayer(false))
    .insert(Name::new("TreeTile"))
    .insert(tree_tile_aabb())
    .id()
}

fn spawn_tree_tile_async(commands: &mut Commands, tile_x: f32, tile_z: f32, lod_level: u32) {
    let thread_pool = AsyncComputeTaskPool::get();
    let transform = Transform::from_xyz(tile_x, 0.0, tile_z);
    let aabb = tree_tile_aabb();
    let task_entity = commands.spawn_empty().id();

    let task = thread_pool.spawn(async move {
        let mut command_queue = CommandQueue::default();
        let mesh = generate_tree_tile_mesh(tile_x, tile_z, lod_level);

        command_queue.push(move |world: &mut World| {
            let (mesh_handle, mat_handle) = {
                let mut system_state = SystemState::<(
                    ResMut<Assets<Mesh>>,
                    ResMut<Assets<ExtendedMaterial<StandardMaterial, TreeMaterialExtension>>>,
                )>::new(world);
                let (mut meshes, mut materials) = system_state.get_mut(world);
                (meshes.add(mesh), materials.add(tree_material()))
            };

            world.entity_mut(task_entity)
                .insert(MaterialMeshBundle {
                    mesh: mesh_handle,
                    material: mat_handle,
                    transform,
                    ..default()
                })
                .insert(Tree)
                .insert(TreeTile { lod_level })
                .insert(ContainsPlayer(false))
                .insert(Name::new("TreeTile"))
                .insert(aabb)
                .remove::<GenTreeTask>();
        });

        command_queue
    });

    commands.entity(task_entity).insert(GenTreeTask(task));
}

fn handle_tree_tasks(mut commands: Commands, mut tree_tasks: Query<&mut GenTreeTask>) {
    for mut task in &mut tree_tasks {
        if let Some(mut commands_queue) = block_on(poll_once(&mut task.0)) {
            commands.append(&mut commands_queue);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_trees(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TreeMaterialExtension>>>,
    mut trees: Query<(Entity, &TreeTile, &Handle<Mesh>, &Transform, &mut ContainsPlayer), With<Tree>>,
    mut grid: Query<&mut TreeGrid>,
    player: Query<&Transform, With<crate::entities::player::Player>>,
) {
    let Ok(player_trans) = player.get_single() else { return };
    let px = player_trans.translation.x;
    let pz = player_trans.translation.z;

    // Get player's current tile
    let player_tile_x = (px / TREE_TILE_SIZE).floor() as i32;
    let player_tile_z = (pz / TREE_TILE_SIZE).floor() as i32;

    if trees.is_empty() && grid.is_empty() {
        // Initial spawn
        let mut tree_grid = TreeGrid(HashMap::new());

        for i in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
            for j in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
                let tile_x = (player_tile_x + i) as f32 * TREE_TILE_SIZE;
                let tile_z = (player_tile_z + j) as f32 * TREE_TILE_SIZE;
                let tile_distance = i.abs().max(j.abs());
                let lod_level = get_lod_level(tile_distance);

                if tile_distance <= LOD_HIGH_DISTANCE {
                    // Spawn high-detail synchronously for nearby tiles
                    tree_grid.0.insert((tile_x as i32, tile_z as i32), TreeTileState::visible(lod_level));
                    spawn_tree_tile(&mut commands, &mut meshes, &mut materials, tile_x, tile_z, lod_level);
                } else {
                    // Spawn async for distant tiles
                    tree_grid.0.insert((tile_x as i32, tile_z as i32), TreeTileState::pending());
                    spawn_tree_tile_async(&mut commands, tile_x, tile_z, lod_level);
                }
            }
        }

        commands.spawn(tree_grid);
    } else {
        let Ok(mut tree_grid) = grid.get_single_mut() else { return };

        // Update grid state for completed async tasks
        for (_, tile, _, trans, _) in trees.iter() {
            let tile_key = (trans.translation.x as i32, trans.translation.z as i32);
            if let Some(state) = tree_grid.0.get(&tile_key) {
                if state.render_state == RenderState::Pending {
                    tree_grid.0.insert(tile_key, TreeTileState::visible(tile.lod_level));
                }
            }
        }

        for (ent, tile, mesh_handle, trans, mut contains_player) in trees.iter_mut() {
            let tile_x = trans.translation.x;
            let tile_z = trans.translation.z;

            // Distance-based despawn
            if (px - tile_x).abs() > DESPAWN_DISTANCE || (pz - tile_z).abs() > DESPAWN_DISTANCE {
                tree_grid.0.remove(&(tile_x as i32, tile_z as i32));
                commands.entity(ent).despawn_recursive();
                continue;
            }

            // Check LOD change
            let tile_i = ((tile_x / TREE_TILE_SIZE).round() as i32) - player_tile_x;
            let tile_j = ((tile_z / TREE_TILE_SIZE).round() as i32) - player_tile_z;
            let tile_distance = tile_i.abs().max(tile_j.abs());
            let new_lod = get_lod_level(tile_distance);

            if new_lod != tile.lod_level {
                // Regenerate mesh with new LOD
                let new_mesh = generate_tree_tile_mesh(tile_x, tile_z, new_lod);
                if let Some(mesh) = meshes.get_mut(mesh_handle) {
                    *mesh = new_mesh;
                }
                commands.entity(ent).insert(TreeTile { lod_level: new_lod });
                tree_grid.0.insert((tile_x as i32, tile_z as i32), TreeTileState::visible(new_lod));
            }

            // Update contains_player
            let in_tile = (px - tile_x).abs() < TREE_TILE_SIZE / 2.0 && (pz - tile_z).abs() < TREE_TILE_SIZE / 2.0;
            if in_tile != contains_player.0 {
                *contains_player = ContainsPlayer(in_tile);

                if in_tile {
                    // Generate new tiles around player
                    for i in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
                        for j in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
                            let new_tile_x = tile_x + i as f32 * TREE_TILE_SIZE;
                            let new_tile_z = tile_z + j as f32 * TREE_TILE_SIZE;
                            let tile_key = (new_tile_x as i32, new_tile_z as i32);

                            // Only spawn if tile doesn't exist in grid
                            let should_spawn = tree_grid.0.get(&tile_key).is_none();

                            if should_spawn {
                                let tile_distance = i.abs().max(j.abs());
                                let lod_level = get_lod_level(tile_distance);
                                tree_grid.0.insert(tile_key, TreeTileState::pending());
                                spawn_tree_tile_async(&mut commands, new_tile_x, new_tile_z, lod_level);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TreeMaterialExtension {
}

impl MaterialExtension for TreeMaterialExtension {

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/tree_shader.wgsl".into()
    }

    // fn fragment_shader() -> ShaderRef {
    //     "shaders/tree_shader.wgsl".into()
    // }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialExtensionKey<TreeMaterialExtension>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let mut pos_position = 0;
        let mut normal_position = 1;
        let mut color_position = 5;
        if let Some(label) = &mut descriptor.label {
            println!("Label is: {}", label);
            if label == "pbr_prepass_pipeline" {
                pos_position = 0;
                normal_position = 3;
                color_position = 7;
            }
        }
        
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(pos_position),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(normal_position),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(color_position),
            // Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            // Mesh::ATTRIBUTE_TANGENT.at_shader_location(4),
            ATTRIBUTE_BASE_Y.at_shader_location(16),
            ATTRIBUTE_STARTING_POSITION.at_shader_location(17),
            ATTRIBUTE_WORLD_POSITION.at_shader_location(18),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

pub struct TreePlugin;

impl Plugin for TreePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, TreeMaterialExtension>>::default());
        app.add_systems(Update, (update_trees, handle_tree_tasks));
    }
}
