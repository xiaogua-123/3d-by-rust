#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput},
    mesh_view_bindings,
    mesh_functions::{get_world_normal, get_world_position},
}

struct OutlineMaterial {
    outline_color: vec4<f32>,
    outline_width: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> outline_mat: OutlineMaterial;

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = get_world_position(in);
    let normal_ws = get_world_normal(in);

    // 沿世界空间法线外扩
    let offset = normal_ws * outline_mat.outline_width * 0.15;
    out.world_position = world_pos + offset;
    out.position = mesh_view_bindings::view.view_proj * vec4<f32>(out.world_position, 1.0);
    out.world_normal = normal_ws;
    out.uv = in.uv;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return outline_mat.outline_color;
}
