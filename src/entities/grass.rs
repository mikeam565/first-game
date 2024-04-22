use std::borrow::BorrowMut;

use crate::entities::{terrain, player};
use crate::{entities, util::perlin::sample_terrain_height};
use bevy::ecs::system::{CommandQueue, SystemState};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MaterialPipeline};
use bevy::render::color;
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayout};
use bevy::render::primitives::Aabb;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{AsBindGroup, GpuArrayBuffer, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError, VertexFormat};
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use bevy::{prelude::*, render::{render_resource::{PrimitiveTopology, Face}, mesh::{self, VertexAttributeValues}}, utils::HashMap};
use noise::{Perlin, NoiseFn};
use rand::distributions::Standard;
use rand::{thread_rng, Rng};
use crate::util::perlin::{PerlinNoiseEntity, self};
use futures_lite::future::poll_once;
use super::player::ContainsPlayer;

// Grass constants
const GRASS_TILE_SIZE_1: f32 = 32.;
const GRASS_TILE_SIZE_2: f32 = 32.; // TODO: like terrain, this causes overlaps if bigger than SIZE_1
const NUM_GRASS_1: u32 = 128; // number of grass blades in one row of a tile
const NUM_GRASS_2: u32 = 32;
const GRASS_BLADE_VERTICES: u32 = 3;
const GRASS_WIDTH: f32 = 0.1;
const GRASS_HEIGHT: f32 = 2.4;
const GRASS_BASE_COLOR_1: [f32;4] = [0.102,0.153,0.,1.];
const GRASS_BASE_COLOR_2: [f32;4] = [0.,0.019,0.,1.];
pub const GRASS_SECOND_COLOR: [f32;4] = [0.079,0.079,0.,1.];
const GRASS_SCALE_FACTOR: f32 = 1.0;
const GRASS_HEIGHT_VARIATION_FACTOR: f32 = 0.2;
const GRASS_STRAIGHTNESS: f32 = 10.0; // for now, as opposed to a curve factor, just modifying denominator for curve calcs
const GRASS_SPACING: f32 = 0.3;
const GRASS_OFFSET: f32 = 0.2;
const ENABLE_WIREFRAME: bool = false;
const WIND_STRENGTH: f32 = 1.;
const WIND_SPEED: f64 = 0.5;
const WIND_CONSISTENCY: f64 = 50.0; //
const WIND_LEAN: f32 = 0.0; // determines how already bent grass will be at 0 wind
const CURVE_POWER: f32 = 1.0; // the linearity / exponentiality of the application/bend of the wind
const DESPAWN_DISTANCE: f32 = (GRID_SIZE_HALF+1) as f32 * GRASS_TILE_SIZE_1 + GRID_SIZE_HALF as f32;
const WIND_SIM_TRIGGER_DISTANCE: f32 = 3. * GRASS_TILE_SIZE_1;
const WIND_SIM_DISTANCE: f32 = WIND_SIM_TRIGGER_DISTANCE - GRASS_TILE_SIZE_1/2.;

const ATTRIBUTE_BASE_Y: MeshVertexAttribute = MeshVertexAttribute::new("BaseY", 988540917, VertexFormat::Float32);
const ATTRIBUTE_STARTING_POSITION: MeshVertexAttribute = MeshVertexAttribute::new("StartingPosition", 988540916, VertexFormat::Float32x3);
const ATTRIBUTE_WORLD_POSITION: MeshVertexAttribute = MeshVertexAttribute::new("WorldPosition", 988540915, VertexFormat::Float32x3);

const GRID_SIZE_HALF: i32 = 8;

fn grass_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        double_sided: false,
        perceptual_roughness: 1.0,
        reflectance: 0.5,
        cull_mode: None,
        opaque_render_method: bevy::pbr::OpaqueRendererMethod::Forward,
        unlit: false,
        ..default()
    }
}

// Grass Component
#[derive(Component)]
pub struct GrassData {
    initial_vertices: Vec<Vec3>,
    initial_positions: Vec<[f32;3]>
}

#[derive(Component,Clone)]
pub struct Grass;

// Grass offsets component

pub fn generate_grass_mesh(
    spawn_x: f32,
    spawn_z: f32,
    density: u32,
    tile_size: f32,
) -> (Mesh, GrassData) {
    let mut grass_offsets = vec![];
    let mut rng = thread_rng();
    let asset_usage = RenderAssetUsages::RENDER_WORLD;// | RenderAssetUsages::MAIN_WORLD;
    // let asset_usage = RenderAssetUsages::RENDER_WORLD;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, asset_usage);
    let mut all_verts: Vec<Vec3> = vec![];
    let mut all_indices: Vec<u32> = vec![];
    let mut all_colors: Vec<[f32; 4]> = vec![];
    let mut blade_number = 0;
    let height_perlin = perlin::grass_perlin();
    let terrain_perlin = perlin::terrain_perlin();
    let start_x = - tile_size/2.;
    let start_z = - tile_size/2.;
    for i in 0..density {
        let x = start_x + i as f32 * tile_size / density as f32;
        for j in 0..density {
            let z = start_z + j as f32 * tile_size / density as f32;
            let rand1 = if GRASS_OFFSET!=0.0 {rng.gen_range(-GRASS_OFFSET..GRASS_OFFSET)} else {0.0};
            let rand2 = if GRASS_OFFSET!=0.0 {rng.gen_range(-GRASS_OFFSET..GRASS_OFFSET)} else {0.0};
            let x_offset = x + rand1;
            let z_offset = z + rand2;
            let y = sample_terrain_height(&terrain_perlin, spawn_x + x_offset, spawn_z + z_offset) - 0.2; // minus small amount to avoid floating
            let blade_height = GRASS_HEIGHT + (height_perlin.get([(spawn_x + x_offset) as f64, (spawn_z + z_offset) as f64]) as f32 * GRASS_HEIGHT_VARIATION_FACTOR);
            if y > terrain::HEIGHT_TEMPERATE_START && y < terrain::HEIGHT_TEMPERATE_END {
                let (mut verts, mut indices) = generate_single_blade_verts(x_offset, y, z_offset, blade_number, blade_height);
                for v in &verts {
                    grass_offsets.push([spawn_x + x_offset,y,spawn_z + z_offset]);
                    let r_color_shift = (terrain_perlin.get([(spawn_x + x_offset) as f64 / 100., (spawn_z + z_offset) as f64 / 100.]) * 0.01) as f32;
                    let mut color = color_gradient_y_based(v.y-y, GRASS_BASE_COLOR_2, GRASS_SECOND_COLOR);
                    color[0] += r_color_shift;
                    all_colors.push(color);
                }
                all_verts.append(&mut verts);
                all_indices.append(&mut indices);
                blade_number += 1;
            }
        }
    }

    generate_grass_geometry(&all_verts, all_indices, &mut mesh, &grass_offsets, all_colors);

    (
        mesh,
        GrassData {
            initial_vertices: all_verts,
            initial_positions: grass_offsets
        }
    )
}

fn grass_data_to_base_data(grass_data: Vec<[f32; 3]>) -> [f32; (GRASS_BLADE_VERTICES * (NUM_GRASS_1*NUM_GRASS_1 + 2)) as usize] {
    let mut arr = [0.; (GRASS_BLADE_VERTICES * (NUM_GRASS_1*NUM_GRASS_1 + 2)) as usize];
    for (i, v) in grass_data.iter().enumerate() {
        arr[i] = v[1];
    }
    arr
}

pub fn generate_grass(
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial,GrassMaterialExtension>>>,
    spawn_x: f32,
    spawn_z: f32,
    density: u32,
    tile_size: f32,
) -> (MaterialMeshBundle<ExtendedMaterial<StandardMaterial, GrassMaterialExtension>>, Grass, GrassData) {
    let (mesh, grass_data) = generate_grass_mesh(spawn_x, spawn_z, density, tile_size);

    let grass_material_ext = GrassMaterialExtension {
    };
    
    let grass_material_std = grass_material();

    let grass_material = ExtendedMaterial {
        base: grass_material_std,
        extension: grass_material_ext
    };

    let bundle = MaterialMeshBundle {
        mesh: meshes.add(mesh),
        material: materials.add(grass_material),
        transform: Transform::from_xyz(spawn_x,0.,spawn_z),
        ..default()
    };

    (
        bundle,
        Grass {},
        grass_data
    )

}

pub fn generate_single_blade_verts(x: f32, y: f32, z: f32, blade_number: u32, blade_height: f32) -> (Vec<Vec3>, Vec<u32>) {
    // For grass with 7 vertices, uncomment t3-6, and uncomment indices
    // vertex transforms
    let t1 = Transform::from_xyz(x, y, z);
    let t2 = Transform::from_xyz(x+GRASS_WIDTH, y, z);
    // let t3 = Transform::from_xyz(x, y+blade_height/3.0, z);
    // let t4 = Transform::from_xyz(x+GRASS_WIDTH, y+blade_height/3.0, z);
    let t5 = Transform::from_xyz(x, y+2.0*blade_height/3.0, z);
    let t6 = Transform::from_xyz(x + GRASS_WIDTH, y+2.0*blade_height/3.0, z);
    let t7 = Transform::from_xyz(x+(GRASS_WIDTH/2.0), y+blade_height, z);
    
    // let mut transforms = vec![t1,t2,t3,t4,t5,t6,t7];
    let mut transforms = vec![t1,t2,t5,t6,t7];
    // let mut transforms = vec![t1,t2,t7];
    let blade_number_shift = blade_number*transforms.len() as u32;
    
    // // physical randomization of grass blades
    // rotate grass randomly around y
    apply_y_rotation(&mut transforms, x, y, z);
    
    // curve the grass all one way
    apply_curve(&mut transforms, x, y, z);

    // rotate grass again
    apply_y_rotation(&mut transforms, x, y, z);
    
    let verts: Vec<Vec3> = transforms.iter().map(|t| t.translation).collect();

    let indices: Vec<u32> = vec![
        blade_number_shift+0, blade_number_shift+1, blade_number_shift+2,
        blade_number_shift+2, blade_number_shift+1, blade_number_shift+3,
        blade_number_shift+2, blade_number_shift+3, blade_number_shift+4,
        // blade_number_shift+4, blade_number_shift+3, blade_number_shift+5,
        // blade_number_shift+4, blade_number_shift+5, blade_number_shift+6,
    ];
    (verts, indices)
}

fn apply_y_rotation(transforms: &mut Vec<Transform>, x: f32, y:f32, z: f32) {
    let y_rotation_point = Vec3::new(x,y,z);
    let rand_rotation = (thread_rng().gen_range(0..628) / 100) as f32;
    for t in transforms {
        t.rotate_around(y_rotation_point, Quat::from_rotation_y(rand_rotation));
    }

    
}

// todo: clean up
fn apply_curve(transforms: &mut Vec<Transform>, x: f32, y:f32, z: f32) {
    let curve_rotation_point = Vec3::new(x + thread_rng().gen_range(0..2) as f32 / 10.0, y, z + thread_rng().gen_range(0..2) as f32 / 10.0);
    let rand_curve = (thread_rng().gen_range(101..110) / 100) as f32;
    for t in transforms {
        t.rotate_around(curve_rotation_point, Quat::from_rotation_z(rand_curve * ((t.translation.y - y) / (GRASS_HEIGHT*GRASS_STRAIGHTNESS))));
    }
}

pub fn generate_grass_geometry(verts: &Vec<Vec3>, vec_indices: Vec<u32>, mesh: &mut Mesh, grass_offsets: &Vec<[f32; 3]>, colors: Vec<[f32; 4]>) {
    let indices = mesh::Indices::U32(vec_indices);

    let vertices: Vec<([f32;3],[f32;3],[f32;2])> = verts.iter().map(|v| { (v.to_array(), [0., 1.,0.], [0.0,0.0])} ).collect();

    let mut positions = Vec::with_capacity(verts.capacity());
    let mut normals = Vec::with_capacity(verts.capacity());
    let bases: Vec<f32> = grass_offsets.iter().map(|x| x[1]).collect();
    // let mut uvs: Vec<[f32; 2]> = Vec::new();
    for (position, normal, uv) in vertices.iter() {
        positions.push(*position);
        normals.push(*normal);
        // uvs.push(*uv);
    }

    mesh.insert_indices(indices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    // mesh.generate_tangents().unwrap();
    mesh.insert_attribute(ATTRIBUTE_BASE_Y, bases);
    mesh.insert_attribute(ATTRIBUTE_STARTING_POSITION, positions);
    mesh.insert_attribute(ATTRIBUTE_WORLD_POSITION, grass_offsets.clone());

}

#[derive(Component)]
struct GrassGrid(HashMap<(i32,i32), bool>);

fn update_grass(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial,GrassMaterialExtension>>>,
    mut grass: Query<(Entity, &Handle<Mesh>, &GrassData, &Transform, &ViewVisibility, &mut ContainsPlayer), With<Grass>>,
    mut grid: Query<&mut GrassGrid>,
    perlin: Res<PerlinNoiseEntity>,
    time: Res<Time>,
    player: Query<(Entity,&Transform),With<entities::player::Player>>,
) {
    let (plyr_e, player_trans) = player.get_single().unwrap();
    let x = player_trans.translation.x;
    let z = player_trans.translation.z;
    if grass.is_empty() {
        let mut grass_grid = GrassGrid(HashMap::new());
        // generate grid of grass
        for i in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
            for j in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
                let a = x + i as f32 * GRASS_TILE_SIZE_1;
                let b = z + j as f32 * GRASS_TILE_SIZE_1;
                grass_grid.0.insert((a as i32, b as i32), true);
                let contains_player = (player_trans.translation.x - a).abs() < GRASS_TILE_SIZE_1/2. && (player_trans.translation.z - b).abs() < GRASS_TILE_SIZE_1/2.;
                let color = if contains_player { Color::RED } else { Color::PURPLE };
                let (main_mat, main_grass, main_data) = generate_grass(&mut meshes, &mut materials, a, b, NUM_GRASS_1, GRASS_TILE_SIZE_1);
                commands.spawn(main_mat)
                    .insert(main_grass)
                    .insert(main_data)
                    .insert(ContainsPlayer(contains_player))
                    // .insert(ShowAabbGizmo {color: Some(color)})
                    ;
            }
        }
        commands.spawn(grass_grid);
    } else {
        let thread_pool = AsyncComputeTaskPool::get();
        let mut grass_grid = grid.get_single_mut().unwrap();
        let elapsed_time = time.elapsed_seconds_f64();
        let mut grass_w_player: Option<Entity> = None;
        for (ent, mh, grass_data, grass_trans, visibility, mut contains_player) in grass.iter_mut() {
            // remove or add ContainsPlayer if applicable
            if (player_trans.translation.x - grass_trans.translation.x).abs() >= GRASS_TILE_SIZE_1/2. || (player_trans.translation.z - grass_trans.translation.z).abs() >= GRASS_TILE_SIZE_1/2. {
                if contains_player.0 {
                    *contains_player = ContainsPlayer(false);
                }
            } else {
                if !contains_player.0 {
                    *contains_player = ContainsPlayer(true);
                    // generate new grass
                    for i in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
                        for j in -GRID_SIZE_HALF..=GRID_SIZE_HALF {
                            let a = grass_trans.translation.x + i as f32 * GRASS_TILE_SIZE_1;
                            let b = grass_trans.translation.z + j as f32 * GRASS_TILE_SIZE_1;
                            if let false = *grass_grid.0.get(&(a as i32,b as i32)).unwrap_or(&false) {
                                grass_grid.0.insert((a as i32, b as i32), true);
                                // todo: async way
                                let transform = Transform::from_xyz(a,0.,b);
    
                                let task_entity = commands.spawn_empty().id();
                                let task = thread_pool.spawn(async move {
                                    let mut command_queue = CommandQueue::default();
                                    let (mesh, grass_data) = generate_grass_mesh(a, b, NUM_GRASS_1, GRASS_TILE_SIZE_1);
    
                                    command_queue.push(move |world: &mut World| {
                                        let (grass_mesh_handle, grass_mat_handle) = {
                                            let mut system_state = SystemState::<(ResMut<Assets<Mesh>>, ResMut<Assets<ExtendedMaterial<StandardMaterial,GrassMaterialExtension>>>)>::new(world);
                                            let (mut meshes, mut mats) = system_state.get_mut(world);
    
                                            (meshes.add(mesh), mats.add(ExtendedMaterial {
                                                base: grass_material(),
                                                extension: GrassMaterialExtension {}
                                            }))
                                        };
    
                                        world.entity_mut(task_entity)
                                        .insert(MaterialMeshBundle {
                                            mesh: grass_mesh_handle,
                                            material: grass_mat_handle,
                                            transform,
                                            ..default()
                                        })
                                        .insert(Grass)
                                        .insert(grass_data)
                                        .insert(ContainsPlayer(false))
                                        // .insert(ShowAabbGizmo {color: Some(Color::PURPLE)})
                                        .remove::<GenGrassTask>();
                                    });
    
                                    command_queue
                                });
    
                                commands.entity(task_entity).insert(GenGrassTask(task)); // spawn a task marked GenGrassTask in the world to be handled by handle_tasks fn when complete

                            //     // old way (sync)
                            //     let (main_mat, main_grass, main_data) = generate_grass(&mut commands, &mut meshes, &mut materials, a, b, NUM_GRASS_1, GRASS_TILE_SIZE_1);
                            //     commands.spawn(main_mat)
                            //         .insert(main_grass)
                            //         .insert(main_data)
                            //         .insert(ContainsPlayer(false))
                            //         // .insert(ShowAabbGizmo {color: Some(Color::PURPLE)})
                            //         ;
                            }
                        }
                    }

                }
            }
            if contains_player.0 {
                grass_w_player = Some(ent);
            }
            // simulate wind only if close enough and if visible
            if (player_trans.translation.x - grass_trans.translation.x).abs() < WIND_SIM_TRIGGER_DISTANCE && (player_trans.translation.z - grass_trans.translation.z).abs() < WIND_SIM_TRIGGER_DISTANCE && visibility.get() {
                // if let Some(mesh) = meshes.get_mut(mh) {
                //     apply_wind(mesh, grass_data, &perlin, elapsed_time, player_trans.translation.xz());
                // }
            } else if (player_trans.translation.x - grass_trans.translation.x).abs() > DESPAWN_DISTANCE || (player_trans.translation.z - grass_trans.translation.z).abs() > DESPAWN_DISTANCE {
                grass_grid.0.insert((grass_trans.translation.x as i32, grass_trans.translation.z as i32), false);
                commands.get_entity(ent).unwrap().despawn_recursive();
            }
        }

        if let Some(grass_w_player) = grass_w_player {
            // update aabb color
            // commands.get_entity(grass_w_player).unwrap().insert(AabbGizmo {color: Some(Color::RED)});
        }

    }
}

fn handle_tasks(mut commands: Commands, mut grass_tasks: Query<&mut GenGrassTask>) {
    for mut task in &mut grass_tasks {
        if let Some(mut commands_queue) = block_on(poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
        }
    }
}

#[derive(Component)]
struct GenGrassTask(Task<CommandQueue>);

#[derive(Resource, Deref)]
struct GrassMeshHandle(Handle<Mesh>);

#[derive(Resource, Deref)]
struct GrassMaterialHandle(Handle<StandardMaterial>);

// 
fn color_gradient_y_based(y: f32, rgba1: [f32; 4], rgba2: [f32; 4]) -> [f32;4] {
    let [r1, g1, b1, a1] = rgba1;
    let [r2, g2, b2, a2] = rgba2;
    let r = r1 + (r2-r1)*(y/GRASS_HEIGHT);
    let g = g1 + (g2-g1)*(y/GRASS_HEIGHT);
    let b = b1 + (b2-b1)*(y/GRASS_HEIGHT);
    let a = a1 + (a2-a1)*(y/GRASS_HEIGHT);
    [r, g, b, a]
}

fn apply_wind(mesh: &mut Mesh, grass: &GrassData, perlin: &PerlinNoiseEntity, time: f64, player_xz: Vec2) {
    let wind_perlin = perlin.wind;
    let pos_attr = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap();
    let VertexAttributeValues::Float32x3(pos_attr) = pos_attr else {
        panic!("Unexpected vertex format, expected Float32x3");
    };
    // for now modify x,z pos. Ideally apply curve instead
    for i in 0..pos_attr.len() {
        let pos = pos_attr.get_mut(i).unwrap(); // current vertex positions
        let initial = grass.initial_vertices.get(i).unwrap(); // initial vertex positions
        let grass_pos = grass.initial_positions.get(i).unwrap(); // initial grass positions

        let [x, y, z] = grass_pos;

        let relative_vertex_height = pos[1] - y;

        let curve_amount = 
            WIND_STRENGTH
            * ((WIND_SIM_DISTANCE - player_xz.distance(Vec2::new(*x,*z))) / WIND_SIM_DISTANCE).powi(2)
            * (sample_noise(&wind_perlin, *x, *z, time) * (relative_vertex_height.powf(CURVE_POWER)/GRASS_HEIGHT.powf(CURVE_POWER)));
        pos[0] = initial.x + curve_amount;
        pos[2] = initial.z + curve_amount;
    }
}

fn sample_noise(perlin: &Perlin, x: f32, z: f32, time: f64) -> f32 {
    WIND_LEAN + perlin.get([WIND_SPEED * time + (x as f64/WIND_CONSISTENCY), WIND_SPEED * time + (z as f64/WIND_CONSISTENCY)]) as f32
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GrassMaterialExtension {
}

impl MaterialExtension for GrassMaterialExtension {

    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/grass_shader.wgsl".into()
    }

    // fn fragment_shader() -> ShaderRef {
    //     "shaders/grass_shader.wgsl".into()
    // }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialExtensionKey<GrassMaterialExtension>,
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

pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial,GrassMaterialExtension>>::default());
        app.add_systems(Update, (update_grass,handle_tasks));
    }
}