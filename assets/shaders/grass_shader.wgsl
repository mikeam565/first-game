#import bevy_pbr::{
    mesh_view_bindings::globals,
    mesh_functions::{get_model_matrix, mesh_position_local_to_clip},
    forward_io::VertexOutput,
}

#import bevy_shader_utils::perlin_noise_2d::perlin_noise_2d


            // Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            // Mesh::ATTRIBUTE_COLOR.at_shader_location(2),

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(3) normal: vec3<f32>,
    @location(7) color: vec4<f32>,
    // @location(3) uv: vec2<f32>,
    // @location(4) tangent: vec3<f32>,
    @location(8) base_y: f32,
    @location(9) starting_position: vec3<f32>,
    @location(10) world_position: vec3<f32>
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;


    var noise = perlin_noise_2d(vec2<f32>(vertex.world_position.x/50.0 + globals.time * 0.5, vertex.world_position.z/50.0 + globals.time * 0.5));

    var new_x = vertex.starting_position.x + noise * ((vertex.position.y-vertex.base_y) / 2.4);
    var new_y = vertex.position.y;
    var new_z = vertex.starting_position.z + noise * ((vertex.position.y-vertex.base_y) / 2.4);
    out.position = mesh_position_local_to_clip(get_model_matrix(vertex.instance_index), vec4<f32>(vec3<f32>(new_x, new_y, new_z), 1.0));
    out.color = vertex.color;
    return out;
}