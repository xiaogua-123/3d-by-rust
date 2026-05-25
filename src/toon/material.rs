use bevy::prelude::*;
use bevy::pbr::MaterialExtension;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;

const TOON_SHADER_PATH: &str = "shaders/toon_shading.wgsl";

/// 卡通着色材质扩展，挂载到 ExtendedMaterial<StandardMaterial, ToonExtension>
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ToonExtension {
    /// Ramp贴图 (256x1, 从左到右暗→亮色阶)
    #[texture(100, dimension = "2d")]
    #[sampler(101)]
    pub ramp_texture: Handle<Image>,

    /// 高光阈值: dot(normal, halfVec) 超过此值才显示高光
    #[uniform(102)]
    pub spec_threshold: f32,
    /// 高光平滑度 (0=纯硬边, 越大越软)
    #[uniform(102)]
    pub spec_smoothness: f32,
    /// 是否启用阶梯着色（关闭则退化为普通PBR）
    #[uniform(102)]
    pub toon_enabled: u32,
    /// 高光颜色 (Vec4: 16字节，与WGSL vec4<f32>对齐一致)
    #[uniform(103)]
    pub spec_color: Vec4,
}

impl Default for ToonExtension {
    fn default() -> Self {
        Self {
            ramp_texture: Handle::default(),
            spec_threshold: 0.8,
            spec_smoothness: 0.01,
            spec_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            toon_enabled: 1,
        }
    }
}

impl MaterialExtension for ToonExtension {
    fn fragment_shader() -> ShaderRef {
        TOON_SHADER_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        TOON_SHADER_PATH.into()
    }
}
