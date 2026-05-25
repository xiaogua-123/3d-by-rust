mod material;
mod outline;
mod ramp;

pub use material::ToonExtension;
pub use outline::{ToonOutline, spawn_outline_meshes};
pub use ramp::generate_ramp_texture;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

/// 全局三渲二设置
#[derive(Resource, Clone)]
#[allow(dead_code)]
pub struct ToonSettings {
    pub ramp_handle: Option<Handle<Image>>,
    pub default_spec_threshold: f32,
    pub default_spec_smoothness: f32,
    pub default_spec_color: Color,
    pub default_outline_color: Color,
    pub default_outline_width: f32,
}

impl Default for ToonSettings {
    fn default() -> Self {
        Self {
            ramp_handle: None,
            default_spec_threshold: 0.8,
            default_spec_smoothness: 0.01,
            default_spec_color: Color::WHITE,
            default_outline_color: Color::BLACK,
            default_outline_width: 0.03,
        }
    }
}

pub struct ToonPlugin;

impl Plugin for ToonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, ToonExtension>>::default(),
        ))
        .init_resource::<ToonSettings>()
        .add_systems(Startup, (generate_ramp_texture, link_ramp_to_settings).chain())
        .add_systems(Update, spawn_outline_meshes);
    }
}

fn link_ramp_to_settings(
    mut settings: ResMut<ToonSettings>,
    ramp_handle: Res<ramp::RampTextureHandle>,
) {
    settings.ramp_handle = Some(ramp_handle.0.clone());
}
