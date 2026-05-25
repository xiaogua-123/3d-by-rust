// ═══════════════════════════════════════════
// 后期 GLB 模型替换方案
// ═══════════════════════════════════════════
// 敌人程序化方块(Cuboid) → 3D 模型:
//   - 关卡敌人(level.rs spawn_zone) → models/characters/enemy_{type}.glb
//     推荐分类: slime.glb / skeleton.glb / ghost.glb / golem.glb
//   - 塔防敌人(td/*.rs) → models/td/enemy_{basic,fast,tank}.glb
//   - 替换时保留 Enemy 组件及其 patrol_points/damage/speed 数据
//   - 模型导入时启用 AnimationGraph 即可支持走路/攻击/死亡动画
//   - 巡逻移动由 enemy_movement 系统驱动（不受模型替换影响）
// ═══════════════════════════════════════════

use bevy::prelude::*;
use crate::combat::{self, AttackDamage, MoveSpeed};
use crate::config::GameplayConfig;
use crate::game_state::{DamagePlayerEvent, GamePhase};
use crate::player::Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Enemy {
    pub patrol_points: Vec<Vec3>,
    pub current_target: usize,
    pub attack_cooldown: Timer,
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Enemy>()
            .add_systems(FixedUpdate, (enemy_movement, enemy_damage_player).run_if(in_state(GamePhase::Playing)));
    }
}

fn enemy_movement(
    time: Res<Time>,
    mut enemy_q: Query<(&mut Transform, &mut Enemy, &MoveSpeed)>,
) {
    for (mut transform, mut enemy, speed) in enemy_q.iter_mut() {
        if enemy.patrol_points.is_empty() {
            continue;
        }

        let target = enemy.patrol_points[enemy.current_target];
        if !combat::move_toward_target(&mut transform, target, speed.0, time.delta_secs(), 0.3) {
            enemy.current_target = (enemy.current_target + 1) % enemy.patrol_points.len();
        }
    }
}

fn enemy_damage_player(
    time: Res<Time>,
    config: Res<GameplayConfig>,
    mut enemy_q: Query<(&Transform, &mut Enemy, &AttackDamage)>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut damage_writer: MessageWriter<DamagePlayerEvent>,
) {
    let Ok(player_t) = player_q.single() else { return };
    let player_pos = player_t.translation;

    for (enemy_t, mut enemy, damage) in enemy_q.iter_mut() {
        enemy.attack_cooldown.tick(time.delta());
        if !enemy.attack_cooldown.is_finished() {
            continue;
        }

        let dist = player_pos.distance(enemy_t.translation);
        if dist < config.enemy_damage_radius {
            damage_writer.write(DamagePlayerEvent(damage.0 as u32));
            enemy.attack_cooldown.reset();
        }
    }
}
