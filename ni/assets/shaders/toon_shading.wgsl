#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct ToonExtension {
    spec_threshold: f32,
    spec_smoothness: f32,
    toon_enabled: u32,
}

struct SpecColorUniform {
    spec_color: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var ramp_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> toon_ext: ToonExtension;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var<uniform> spec_color_uniform: SpecColorUniform;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    let pbr_lit = apply_pbr_lighting(pbr_input);

    if toon_ext.toon_enabled == 0u {
        var pbr_out: FragmentOutput;
        pbr_out.color = main_pass_post_lighting_processing(pbr_input, pbr_lit);
        return pbr_out;
    }

    // 🔧 诊断：输出apply_pbr_lighting结果，看光照是否正常
    var out: FragmentOutput;
    out.color = vec4<f32>(pbr_lit.rgb, 1.0);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
