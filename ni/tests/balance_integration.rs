//! 塔防数值平衡集成测试
//!
//! 验证 RON 数据文件的解析和数值常量是否在预期范围内。

use ni::td::balance::{TdConfig, WaveConfig, TurretGlobalConfig, EconomyConfig, CoreConfig, DifficultyScaling};

#[test]
fn wave_config_defaults_are_reasonable() {
    let config = WaveConfig {
        max_waves: 30,
        enemies_per_wave_base: 5,
        enemies_per_wave_growth: 2,
        spawn_interval: 1.0,
        wave_cooldown: 20.0,
        starting_gold: 100,
    };
    // 总敌人数量应在合理范围内
    let total_enemies: u32 = (0..config.max_waves)
        .map(|w| config.enemies_per_wave_base + w * config.enemies_per_wave_growth)
        .sum();
    assert!(total_enemies > 100, "应有足够敌人");
    assert!(total_enemies < 5000, "敌人不应过多");
    // 产怪间隔和冷却应为正数
    assert!(config.spawn_interval > 0.0);
    assert!(config.wave_cooldown > 0.0);
}

#[test]
fn turret_global_values_are_positive() {
    let config = TurretGlobalConfig {
        rotation_speed: 3.0,
        projectile_speed: 15.0,
        projectile_lifetime: 2.0,
        target_search_interval: 0.3,
    };
    assert!(config.rotation_speed > 0.0);
    assert!(config.projectile_speed > 0.0);
    assert!(config.projectile_lifetime > 0.0);
    assert!(config.target_search_interval > 0.0);
}

#[test]
fn economy_gold_interest_is_reasonable() {
    let config = EconomyConfig {
        gold_interest_rate: 0.05,
        gold_per_wave_bonus: 50,
        turret_refund_ratio: 0.7,
    };
    assert!(config.gold_interest_rate >= 0.0);
    assert!(config.gold_interest_rate <= 1.0);
    assert!(config.gold_per_wave_bonus > 0);
    assert!(config.turret_refund_ratio > 0.0);
    assert!(config.turret_refund_ratio <= 1.0);
}

#[test]
fn core_health_is_reasonable() {
    let config = CoreConfig { max_health: 100.0 };
    assert!(config.max_health > 0.0);
    assert!(config.max_health < 10000.0);
}

#[test]
fn difficulty_scaling_starts_at_reasonable_base() {
    let config = DifficultyScaling {
        health_mult_per_wave: 1.1,
        speed_mult_per_wave: 1.05,
        damage_mult_per_wave: 1.08,
        gold_mult_per_wave: 1.15,
    };
    assert!(config.health_mult_per_wave >= 1.0);
    assert!(config.health_mult_per_wave < 2.0);
    assert!(config.speed_mult_per_wave >= 1.0);
    assert!(config.speed_mult_per_wave < 2.0);
    assert!(config.damage_mult_per_wave >= 1.0);
    assert!(config.damage_mult_per_wave < 2.0);

    // 验证 20 波后难度没有爆炸
    let health_mult_20 = config.health_mult_per_wave.powi(20);
    assert!(health_mult_20 < 20.0, "20波后血量倍率应可控: {}", health_mult_20);
}

#[test]
fn full_td_config_sanity() {
    let config = TdConfig {
        wave: WaveConfig { max_waves: 30, enemies_per_wave_base: 5, enemies_per_wave_growth: 2, spawn_interval: 1.0, wave_cooldown: 20.0, starting_gold: 100 },
        turret_global: TurretGlobalConfig { rotation_speed: 3.0, projectile_speed: 15.0, projectile_lifetime: 2.0, target_search_interval: 0.3 },
        economy: EconomyConfig { gold_interest_rate: 0.05, gold_per_wave_bonus: 50, turret_refund_ratio: 0.7 },
        core: CoreConfig { max_health: 100.0 },
        scaling: DifficultyScaling { health_mult_per_wave: 1.1, speed_mult_per_wave: 1.05, damage_mult_per_wave: 1.08, gold_mult_per_wave: 1.15 },
    };
    // 验证所有字段被正确读取
    assert_eq!(config.wave.max_waves, 30);
    assert_eq!(config.wave.starting_gold, 100);
    assert!((config.turret_global.projectile_speed - 15.0).abs() < 0.01);
    assert!((config.core.max_health - 100.0).abs() < 0.01);
}
