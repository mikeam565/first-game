use std::f32::consts::PI;

use bevy::{prelude::*, render::{render_resource::PrimitiveTopology, mesh::{self, VertexAttributeValues}}, utils::HashMap};
use noise::{Perlin, NoiseFn};
use rand::{thread_rng, Rng};
use crate::util::perlin::PerlinNoiseEntity;

// Grass constants
const GRASS_BLADES: u32 = 65536;
const NUM_GRASS_X: u32 = 256;
const NUM_GRASS_Y: u32 = 256;
const GRASS_BLADE_VERTICES: u32 = 7;
const GRASS_WIDTH: f32 = 0.1;
const GRASS_HEIGHT: f32 = 3.0;
const GRASS_SCALE_FACTOR: f32 = 0.4;
const GRASS_HEIGHT_VARIATION_FACTOR: f32 = 0.4;
const GRASS_STRAIGHTNESS: f32 = 10.0; // for now, as opposed to a curve factor, just modifying denominator for curve calcs
const GRASS_SPACING: f32 = 0.2;
const GRASS_OFFSET: f32 = 0.1;
const ENABLE_WIREFRAME: bool = false;
const WIND_STRENGTH: f32 = 1.0;
const WIND_SPEED: f64 = 0.5;
const WIND_CONSISTENCY: f64 = 25.0; //

// Grass Component
#[derive(Component,Clone)]
struct Grass {
    initial_vertices: Vec<Vec3>,
    initial_positions: Vec<[f32;2]>
}

// Grass offsets component

pub fn generate_grass(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let mut grass_offsets = vec![];
    let mut rng = thread_rng();
    let mut mesh = if !ENABLE_WIREFRAME { Mesh::new(PrimitiveTopology::TriangleList) } else { Mesh::new(PrimitiveTopology::LineList)};
    let mut all_verts: Vec<Vec3> = vec![];
    let mut all_indices: Vec<u32> = vec![];
    let mut blade_number = 0;
    let perlin = PerlinNoiseEntity::new();
    let height_perlin = perlin.grass_height;
    for i in 0..NUM_GRASS_X {
        let x = i as f32;
        for j in 0..NUM_GRASS_Y {
            let z = j as f32;
            let rand1 = if GRASS_OFFSET!=0.0 {rng.gen_range(-GRASS_OFFSET..GRASS_OFFSET)} else {0.0};
            let rand2 = if GRASS_OFFSET!=0.0 {rng.gen_range(-GRASS_OFFSET..GRASS_OFFSET)} else {0.0};
            let x_offset = x * GRASS_SPACING + rand1;
            let z_offset = z * GRASS_SPACING + rand2;
            let blade_height = GRASS_HEIGHT + (height_perlin.get([x_offset as f64, z_offset as f64]) as f32 * GRASS_HEIGHT_VARIATION_FACTOR);
            // let blade_height = GRASS_HEIGHT;
            let (mut verts, mut indices) = generate_single_blade_verts(x_offset, z_offset, blade_number, blade_height);
            for _ in 0..verts.len() {
                grass_offsets.push([x_offset,z_offset]);
            }
            all_verts.append(&mut verts);
            all_indices.append(&mut indices);
            blade_number += 1;
        }
    }

    generate_grass_geometry(&all_verts, all_indices, &mut mesh);

    let grass_material = StandardMaterial {
        base_color: Color::GREEN.into(),
        double_sided: false,
        cull_mode: None,
        ..default()
    };
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh),
        material: materials.add(grass_material),
        transform: Transform::from_xyz(-((NUM_GRASS_X/8) as f32), 0.0, -((NUM_GRASS_Y/8) as f32)).with_scale(Vec3::new(1.0,GRASS_SCALE_FACTOR,1.0)),
        ..default()
    })
    .insert(Grass {
        initial_vertices: all_verts,
        initial_positions: grass_offsets
    });
}

fn generate_single_blade_verts(x: f32, z: f32, blade_number: u32, blade_height: f32) -> (Vec<Vec3>, Vec<u32>) {
    let blade_number_shift = blade_number*GRASS_BLADE_VERTICES;
    
    let t1 = Transform::from_xyz(x, 0.0, z);
    let t2 = Transform::from_xyz(x+GRASS_WIDTH, 0.0, z);
    let t3 = Transform::from_xyz(x, blade_height/3.0, z);
    let t4 = Transform::from_xyz(x+GRASS_WIDTH, blade_height/3.0, z);
    let t5 = Transform::from_xyz(x, 2.0*blade_height/3.0, z);
    let t6 = Transform::from_xyz(x + GRASS_WIDTH, 2.0*blade_height/3.0, z);
    let t7 = Transform::from_xyz(x+(GRASS_WIDTH/2.0), blade_height, z);

    let mut transforms = vec![t1,t2,t3,t4,t5,t6,t7];
    
    // curve the grass all one way
    apply_curve(&mut transforms, x, z);

    // rotate grass randomly around y
    apply_y_rotation(&mut transforms, x, z);
    
    let verts: Vec<Vec3> = transforms.iter().map(|t| t.translation).collect();

    // can't think of non-manual way to address indices
    let indices: Vec<u32> = vec![
        blade_number_shift+0, blade_number_shift+1, blade_number_shift+2,
        blade_number_shift+2, blade_number_shift+1, blade_number_shift+3,
        blade_number_shift+2, blade_number_shift+3, blade_number_shift+4,
        blade_number_shift+4, blade_number_shift+2, blade_number_shift+3,
        blade_number_shift+4, blade_number_shift+3, blade_number_shift+5,
        blade_number_shift+4, blade_number_shift+5, blade_number_shift+6,
    ];
    (verts, indices)
}

fn apply_y_rotation(transforms: &mut Vec<Transform>, x: f32, z: f32) {
    let y_rotation_point = Vec3::new(x,0.0,z);
    let rand_rotation = (thread_rng().gen_range(0..628) / 100) as f32;
    for t in transforms {
        t.rotate_around(y_rotation_point, Quat::from_rotation_y(rand_rotation));
    }
}

// todo: clean up
fn apply_curve(transforms: &mut Vec<Transform>, x: f32, z: f32) {
    let curve_rotation_point = Vec3::new(x + thread_rng().gen_range(0..2) as f32 / 10.0, 0.0, z + thread_rng().gen_range(0..2) as f32 / 10.0);
    let rand_curve = (thread_rng().gen_range(101..110) / 100) as f32;
    for t in transforms {
        t.rotate_around(curve_rotation_point, Quat::from_rotation_z(rand_curve * (t.translation.y / (GRASS_HEIGHT*GRASS_STRAIGHTNESS))));
    }
}

fn generate_grass_geometry(verts: &Vec<Vec3>, vec_indices: Vec<u32>, mesh: &mut Mesh) {
    let indices = mesh::Indices::U32(vec_indices);

    // for now, normals are same as verts, and UV is [1.0,1.0]
    let vertices: Vec<([f32;3],[f32;3],[f32;2])> = verts.iter().map(|v| { (v.to_array(), v.to_array(), [1.0,1.0])} ).collect();

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in vertices.iter() {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

fn update_grass(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut grass: Query<(&Handle<Mesh>, &Grass)>,
    mut perlin: Query<&PerlinNoiseEntity>,
    time: Res<Time>
) {
    let time = time.elapsed_seconds_f64();
    let (mesh_handle, grass) = grass.get_single_mut().unwrap();
    let mesh = meshes.get_mut(mesh_handle).unwrap();
    let perlin = perlin.get_single_mut().unwrap();
    apply_wind(mesh, grass, perlin, time);
}

fn apply_wind(mesh: &mut Mesh, grass: &Grass, perlin: &PerlinNoiseEntity, time: f64) {
    let wind_perlin = perlin.wind;
    let pos_attr = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap();
    let VertexAttributeValues::Float32x3(pos_attr) = pos_attr else {
        panic!("Unexpected vertex format, expected Float32x3");
    };
    for i in 0..pos_attr.len() {
        let pos = pos_attr.get_mut(i).unwrap();
        let initial = grass.initial_vertices.get(i).unwrap();
        let grass_pos = grass.initial_positions.get(i).unwrap();
        let curve_amount = WIND_STRENGTH * (sample_noise(&wind_perlin, grass_pos[0], grass_pos[1],  time) * (pos[1] / GRASS_HEIGHT));
        pos[0] = initial.x + curve_amount;
        pos[2] = initial.z + curve_amount;
    }
}

fn sample_noise(perlin: &Perlin, x: f32, z: f32, time: f64) -> f32 {
    let noise = perlin.get([WIND_SPEED * time + (x as f64/WIND_CONSISTENCY), WIND_SPEED * time + (z as f64/WIND_CONSISTENCY)]) as f32;
    noise
}

pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_grass);
    }
}