use std::f32::consts::PI;
use crate::{entities, util::perlin::sample_terrain_height};
use bevy::{prelude::*, render::{render_resource::{PrimitiveTopology, Face}, mesh::{self, VertexAttributeValues}}, utils::HashMap};
use noise::{Perlin, NoiseFn};
use rand::{thread_rng, Rng};
use crate::util::perlin::{PerlinNoiseEntity, self};

// Grass constants
const GRASS_BLADES: u32 = 65536;
const NUM_GRASS_X: u32 = 512;
const NUM_GRASS_Y: u32 = 512;
const GRASS_BLADE_VERTICES: u32 = 3;
const GRASS_WIDTH: f32 = 0.16;
const GRASS_HEIGHT: f32 = 3.0;
const GRASS_BASE_COLOR_1: [f32;4] = [0.102,0.153,0.,1.];
const GRASS_BASE_COLOR_2: [f32;4] = [0.,0.019,0.,1.];
const GRASS_SECOND_COLOR: [f32;4] = [0.079,0.079,0.,1.];
const GRASS_SCALE_FACTOR: f32 = 1.0;
const GRASS_HEIGHT_VARIATION_FACTOR: f32 = 0.2;
const GRASS_STRAIGHTNESS: f32 = 10.0; // for now, as opposed to a curve factor, just modifying denominator for curve calcs
const GRASS_SPACING: f32 = 0.2;
const GRASS_OFFSET: f32 = 0.1;
const ENABLE_WIREFRAME: bool = false;
const WIND_STRENGTH: f32 = 0.5;
const WIND_SPEED: f64 = 0.5;
const WIND_CONSISTENCY: f64 = 10.0; //
const WIND_LEAN: f32 = 0.0; // determines how already bent grass will be at 0 wind
const CURVE_POWER: f32 = 1.0; // the linearity / exponentiality of the application/bend of the wind

// Grass Component
#[derive(Resource)]
struct GrassData {
    initial_vertices: Vec<Vec3>,
    initial_positions: Vec<[f32;3]>
}

#[derive(Component,Clone)]
struct Grass;

// Grass offsets component

pub fn generate_grass(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let mut grass_offsets = vec![];
    let mut rng = thread_rng();
    let mut mesh = if !entities::util::ENABLE_WIREFRAME { Mesh::new(PrimitiveTopology::TriangleList) } else { Mesh::new(PrimitiveTopology::LineList)};
    let mut all_verts: Vec<Vec3> = vec![];
    let mut all_indices: Vec<u32> = vec![];
    let mut blade_number = 0;
    let height_perlin = perlin::grass_perlin();
    let terrain_perlin = perlin::terrain_perlin();
    for i in 0..NUM_GRASS_X {
        let x = i as f32;
        for j in 0..NUM_GRASS_Y {
            let z = j as f32;
            let rand1 = if GRASS_OFFSET!=0.0 {rng.gen_range(-GRASS_OFFSET..GRASS_OFFSET)} else {0.0};
            let rand2 = if GRASS_OFFSET!=0.0 {rng.gen_range(-GRASS_OFFSET..GRASS_OFFSET)} else {0.0};
            let x_offset = x * GRASS_SPACING + rand1;
            let z_offset = z * GRASS_SPACING + rand2;
            let y = sample_terrain_height(&terrain_perlin, x_offset, z_offset) - 0.2; // minus small amount to avoid floating
            let blade_height = GRASS_HEIGHT + (height_perlin.get([x_offset as f64, z_offset as f64]) as f32 * GRASS_HEIGHT_VARIATION_FACTOR);
            let (mut verts, mut indices) = generate_single_blade_verts(x_offset, y, z_offset, blade_number, blade_height);
            for _ in 0..verts.len() {
                grass_offsets.push([x_offset,y,z_offset]);
            }
            all_verts.append(&mut verts);
            all_indices.append(&mut indices);
            blade_number += 1;
        }
    }

    generate_grass_geometry(&all_verts, all_indices, &mut mesh, &grass_offsets);

    let grass_material = StandardMaterial {
        base_color: Color::WHITE,
        double_sided: false,
        perceptual_roughness: 1.0,
        reflectance: 0.5,
        diffuse_transmission: 0.6,
        cull_mode: Some(Face::Back),
        ..default()
    };
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh),
        material: materials.add(grass_material),
        transform: Transform::from_xyz(0.0,200.,0.0),
        ..default()
    })
    .insert(Grass {});
    commands.insert_resource(GrassData {
        initial_vertices: all_verts,
        initial_positions: grass_offsets
    });
}

fn generate_single_blade_verts(x: f32, y: f32, z: f32, blade_number: u32, blade_height: f32) -> (Vec<Vec3>, Vec<u32>) {
    // For grass with 7 vertices, uncomment t3-6, and uncomment indices
    let blade_number_shift = blade_number*GRASS_BLADE_VERTICES;
    // vertex transforms
    let t1 = Transform::from_xyz(x, y, z);
    let t2 = Transform::from_xyz(x+GRASS_WIDTH, y, z);
    // let t3 = Transform::from_xyz(x, y+blade_height/3.0, z);
    // let t4 = Transform::from_xyz(x+GRASS_WIDTH, y+blade_height/3.0, z);
    // let t5 = Transform::from_xyz(x, y+2.0*blade_height/3.0, z);
    // let t6 = Transform::from_xyz(x + GRASS_WIDTH, y+2.0*blade_height/3.0, z);
    let t7 = Transform::from_xyz(x+(GRASS_WIDTH/2.0), y+blade_height, z);

    // let mut transforms = vec![t1,t2,t3,t4,t5,t6,t7];
    let mut transforms = vec![t1,t2,t7];
    
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
        // blade_number_shift+2, blade_number_shift+1, blade_number_shift+3,
        // blade_number_shift+2, blade_number_shift+3, blade_number_shift+4,
        // blade_number_shift+4, blade_number_shift+2, blade_number_shift+3,
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
    let curve_rotation_point = Vec3::new(x + thread_rng().gen_range(0..2) as f32 / 10.0, y - GRASS_HEIGHT, z + thread_rng().gen_range(0..2) as f32 / 10.0);
    let rand_curve = (thread_rng().gen_range(101..110) / 100) as f32;
    for t in transforms {
        t.rotate_around(curve_rotation_point, Quat::from_rotation_z(rand_curve * (t.translation.y / (GRASS_HEIGHT*GRASS_STRAIGHTNESS))));
    }
}

fn generate_grass_geometry(verts: &Vec<Vec3>, vec_indices: Vec<u32>, mesh: &mut Mesh, grass_offsets: &Vec<[f32; 3]>) {
    let indices = mesh::Indices::U32(vec_indices);

    // for now, normals are same as verts, and UV is [0.0,0.0]
    let vertices: Vec<([f32;3],[f32;3],[f32;2])> = verts.iter().map(|v| { (v.to_array(), v.to_array(), [0.0,0.0])} ).collect();

    let mut positions = Vec::with_capacity(verts.capacity());
    let mut normals = Vec::with_capacity(verts.capacity());
    // let mut uvs: Vec<[f32; 2]> = Vec::new();
    for (position, normal, uv) in vertices.iter() {
        positions.push(*position);
        normals.push(*normal);
        // uvs.push(*uv);
    }

    let colors: Vec<[f32; 4]> = generate_vertex_colors(&positions, grass_offsets);

    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

fn update_grass(
    mut meshes: ResMut<Assets<Mesh>>,
    mut grass: Query<&Handle<Mesh>, With<Grass>>,
    grass_data: Res<GrassData>,
    perlin: Res<PerlinNoiseEntity>,
    time: Res<Time>
) {
    let time = time.elapsed_seconds_f64();
    let mesh_handle = grass.get_single_mut().unwrap();
    let mesh = meshes.get_mut(mesh_handle).unwrap();
    apply_wind(mesh, grass_data.as_ref(), &perlin, time);
}

fn generate_vertex_colors(positions: &Vec<[f32; 3]>, grass_offsets: &Vec<[f32; 3]>) -> Vec<[f32; 4]> {
    positions.iter().enumerate().map(|(i,[x,y,z])| {
        let [_, base_y, _] = grass_offsets.get(i).unwrap();
        let modified_y = *y - base_y;
        // [0.03*modified_y + 0.07,0.128,0.106 * -modified_y, 1.]
        color_gradient_y_based(modified_y, GRASS_BASE_COLOR_2, GRASS_SECOND_COLOR)
    }).collect()
}

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

fn apply_wind(mesh: &mut Mesh, grass: &GrassData, perlin: &PerlinNoiseEntity, time: f64) {
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

        let curve_amount = WIND_STRENGTH * (sample_noise(&wind_perlin, *x, *z, time) * (relative_vertex_height.powf(CURVE_POWER)/GRASS_HEIGHT.powf(CURVE_POWER)));
        pos[0] = initial.x + curve_amount;
        pos[2] = initial.z + curve_amount;
    }
}

fn sample_noise(perlin: &Perlin, x: f32, z: f32, time: f64) -> f32 {
    WIND_LEAN + perlin.get([WIND_SPEED * time + (x as f64/WIND_CONSISTENCY), WIND_SPEED * time + (z as f64/WIND_CONSISTENCY)]) as f32
}

fn add_grass(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    generate_grass(&mut commands, &mut meshes, &mut materials);
}

pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_grass);
        app.add_systems(Update, update_grass);
    }
}