use bevy::{ecs::relationship::Relationship, prelude::*};

/// 标记组件：有此标记的实体会自动生成描边子实体
#[derive(Component, Clone, Default)]
pub struct ToonOutline;

/// 描边实体标记（内部用）
#[derive(Component)]
pub(crate) struct OutlineEntity;

/// 系统：为有 ToonOutline 组件的实体自动生成描边子实体
/// 简单方案：使用 unlit StandardMaterial + 略微放大
pub fn spawn_outline_meshes(
    mut commands: Commands,
    _settings: Res<crate::toon::ToonSettings>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    query: Query<
        (Entity, &Mesh3d),
        (With<ToonOutline>, Without<OutlineEntity>),
    >,
    outline_parents: Query<&ChildOf, With<OutlineEntity>>,
) {
    for (entity, mesh_handle) in query.iter() {
        let already_has_outline = outline_parents
            .iter()
            .any(|parent| parent.get() == entity);

        if already_has_outline {
            continue;
        }

        let outline_mat = std_materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            alpha_mode: AlphaMode::Opaque,
            depth_bias: 1.0, // 防止与主体 z-fighting
            ..default()
        });

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                mesh_handle.clone(),
                MeshMaterial3d(outline_mat),
                Transform::from_scale(Vec3::new(1.05, 1.05, 1.05)),
                Visibility::default(),
                OutlineEntity,
                Name::new("Outline"),
            ));
        });
    }
}
