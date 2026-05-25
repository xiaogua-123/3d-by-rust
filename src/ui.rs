use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use crate::game_state::*;
use crate::level::{GameLevel, LevelConfig};
use crate::td;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, (
            setup_fonts,
            main_menu_ui.run_if(in_state(GamePhase::MainMenu)),
            hud_ui.run_if(in_state(GamePhase::Playing)),
            td_hud_ui.run_if(in_state(GamePhase::Playing)),
            pause_ui.run_if(in_state(GamePhase::Paused)),
            game_over_ui.run_if(in_state(GamePhase::GameOver)),
            level_complete_ui.run_if(in_state(GamePhase::LevelComplete)),
            td_victory_ui.run_if(in_state(GamePhase::LevelComplete)),
        ));
    }
}

// --- UI helpers ---

fn overlay_ui(ctx: &egui::Context, fill: egui::Color32, top_space: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(egui::Frame { fill, ..Default::default() })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(top_space);
                add_contents(ui);
            });
        });
}

fn action_button(ui: &mut egui::Ui, text: &str, font_size: f32, width: f32, height: f32) -> bool {
    ui.add_sized(
        egui::vec2(width, height),
        egui::Button::new(egui::RichText::new(text).size(font_size)),
    )
    .clicked()
}

fn overlay_heading(ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32) {
    ui.heading(egui::RichText::new(text).size(size).color(color));
}

// --- Fonts ---

fn setup_fonts(
    mut contexts: EguiContexts,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "msyh".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/msyh.ttf")).into(),
    );

    // Put msyh as primary, keep default fonts as fallback for missing glyphs
    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_insert_with(Vec::new);
    proportional.insert(0, "msyh".to_owned());

    let monospace = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_insert_with(Vec::new);
    monospace.insert(0, "msyh".to_owned());

    ctx.set_fonts(fonts);
    *done = true;
}

// --- Screens ---

fn main_menu_ui(
    mut contexts: EguiContexts,
    mut phase: ResMut<NextState<GamePhase>>,
    mut start_writer: MessageWriter<StartGameEvent>,
    mut exit_writer: MessageWriter<AppExit>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_ui(ctx, egui::Color32::from_rgba_premultiplied(10, 10, 20, 230), 120.0, |ui| {
        overlay_heading(ui, "NI", 72.0, egui::Color32::from_rgb(255, 200, 50));
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("从零开始的 3D 冒险")
                .size(20.0)
                .color(egui::Color32::from_rgb(180, 180, 200)),
        );

        ui.add_space(60.0);

        if action_button(ui, "开始游戏", 24.0, 200.0, 50.0) {
            start_writer.write(StartGameEvent);
        }
        ui.add_space(16.0);
        if action_button(ui, "多人聊天", 22.0, 200.0, 45.0) {
            phase.set(GamePhase::MultiplayerChat);
        }
        ui.add_space(16.0);
        if action_button(ui, "退出", 18.0, 200.0, 40.0) {
            exit_writer.write(AppExit::Success);
        }

        ui.add_space(30.0);
        ui.label(
            egui::RichText::new("WASD 移动 | 空格跳跃 | 鼠标视角 | ESC 暂停")
                .size(14.0)
                .color(egui::Color32::GRAY),
        );
        ui.label(
            egui::RichText::new("收集所有金色光球来完成关卡!")
                .size(14.0)
                .color(egui::Color32::GRAY),
        );
    });
}

fn hud_ui(
    mut contexts: EguiContexts,
    score: Res<Score>,
    health: Res<PlayerHealth>,
    collectibles: Res<LevelCollectibles>,
    level: Res<LevelConfig>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::TopBottomPanel::top("hud")
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(0, 0, 0, 120),
            inner_margin: egui::Margin::same(12),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(level.current_level.display_name())
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.separator();

                let hearts_str: String = (0..health.max)
                    .map(|i| if i < health.current { "❤" } else { "🖤" })
                    .collect();
                ui.label(egui::RichText::new(hearts_str).size(18.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("分数: {}", score.0))
                            .size(18.0)
                            .color(egui::Color32::YELLOW),
                    );
                    ui.separator();

                    if collectibles.total > 0 {
                        ui.label(
                            egui::RichText::new(format!(
                                "收集: {}/{}",
                                collectibles.collected, collectibles.total
                            ))
                            .size(18.0)
                            .color(egui::Color32::from_rgb(255, 200, 100)),
                        );
                    }
                });
            });
        });
}

fn pause_ui(
    mut contexts: EguiContexts,
    mut phase: ResMut<NextState<GamePhase>>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_ui(ctx, egui::Color32::from_rgba_premultiplied(0, 0, 0, 160), 180.0, |ui| {
        overlay_heading(ui, "暂停", 48.0, egui::Color32::WHITE);
        ui.add_space(40.0);

        if action_button(ui, "继续游戏", 22.0, 200.0, 50.0) {
            phase.set(GamePhase::Playing);
        }
        ui.add_space(16.0);
        if action_button(ui, "返回主菜单", 18.0, 200.0, 40.0) {
            main_menu_writer.write(MainMenuEvent);
        }
    });
}

fn game_over_ui(
    mut contexts: EguiContexts,
    score: Res<Score>,
    mut restart_writer: MessageWriter<RestartGameEvent>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_ui(ctx, egui::Color32::from_rgba_premultiplied(20, 0, 0, 200), 160.0, |ui| {
        overlay_heading(ui, "游戏结束", 52.0, egui::Color32::RED);
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(format!("最终分数: {}", score.0))
                .size(24.0)
                .color(egui::Color32::YELLOW),
        );
        ui.add_space(40.0);

        if action_button(ui, "重新开始", 22.0, 200.0, 50.0) {
            restart_writer.write(RestartGameEvent);
        }
        ui.add_space(16.0);
        if action_button(ui, "返回主菜单", 18.0, 200.0, 40.0) {
            main_menu_writer.write(MainMenuEvent);
        }
    });
}

// ═══════════════════════════════════════════
// 塔防 HUD (仅在 TD 关卡显示)
// ═══════════════════════════════════════════

fn td_hud_ui(
    mut contexts: EguiContexts,
    level: Res<LevelConfig>,
    gold: Res<td::TdGold>,
    wave_state: Res<td::TdWaveState>,
    config: Res<td::TdWaveConfig>,
    core_q: Query<&td::DefenseCore>,
    mut start_wave_writer: MessageWriter<td::StartNextWaveEvent>,
    mut purchase_writer: MessageWriter<td::PurchaseTurretEvent>,
    player_q: Query<&Transform, With<crate::player::Player>>,
) {
    // 只在 TD 关卡显示
    if level.current_level != GameLevel::Level5 {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };
    let player_pos = player_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // 顶部资源栏
    egui::TopBottomPanel::top("td_hud")
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(0, 0, 0, 140),
            inner_margin: egui::Margin::same(12),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("塔防试炼")
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 180, 50)),
                );
                ui.separator();

                // 金币
                ui.label(
                    egui::RichText::new(format!("金币: {}", gold.0))
                        .size(16.0)
                        .color(egui::Color32::from_rgb(255, 215, 0)),
                );
                ui.separator();

                // 波次信息
                let phase_text = match wave_state.phase {
                    td::WavePhase::Waiting => format!(
                        "下一波: {} ({}s)",
                        wave_state.current_wave + 1,
                        (config.wave_cooldown - wave_state.wave_timer.elapsed_secs()).max(0.0) as u32
                    ),
                    td::WavePhase::Spawning => format!(
                        "第 {} 波 生成中... 剩余: {}",
                        wave_state.current_wave,
                        wave_state.enemies_to_spawn + wave_state.enemies_alive
                    ),
                    td::WavePhase::Active => format!(
                        "第 {} 波 战斗中! 剩余敌人: {}",
                        wave_state.current_wave, wave_state.enemies_alive
                    ),
                    td::WavePhase::Complete => "全部波次完成!".to_string(),
                };
                ui.label(
                    egui::RichText::new(phase_text)
                        .size(15.0)
                        .color(egui::Color32::WHITE),
                );
                ui.separator();

                // 防御核心生命
                if let Ok(core) = core_q.single() {
                    let ratio = core.current_health / core.max_health;
                    let color = if ratio > 0.5 {
                        egui::Color32::GREEN
                    } else if ratio > 0.25 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "核心: {:.0}/{:.0}",
                            core.current_health, core.max_health
                        ))
                        .size(15.0)
                        .color(color),
                    );
                }
                ui.separator();

                // 手动开始下一波按钮
                if wave_state.phase == td::WavePhase::Waiting
                    && wave_state.current_wave < config.max_waves
                {
                    if ui
                        .add_sized(
                            egui::vec2(120.0, 28.0),
                            egui::Button::new(
                                egui::RichText::new("开始下一波").size(14.0),
                            ),
                        )
                        .clicked()
                    {
                        start_wave_writer.write(td::StartNextWaveEvent);
                    }
                }
            });
        });

    // 右侧建造面板
    egui::SidePanel::right("td_shop")
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(10, 10, 20, 200),
            inner_margin: egui::Margin::same(10),
            ..Default::default()
        })
        .min_width(180.0)
        .show(ctx, |ui| {
            ui.heading(
                egui::RichText::new("建造炮台")
                    .size(16.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(8.0);

            // 炮塔类型列表
            shop_button(
                ui,
                "基础炮台",
                "50 金币\n伤害: 10 | 射速: 1s",
                gold.0 >= td::TurretType::Basic.cost(),
                || {
                    purchase_writer.write(td::PurchaseTurretEvent {
                        turret_type: td::TurretType::Basic,
                        position: player_pos,
                    });
                },
            );
            ui.add_space(6.0);

            shop_button(
                ui,
                "速射炮台",
                "100 金币\n伤害: 5 | 射速: 0.3s",
                gold.0 >= td::TurretType::Rapid.cost(),
                || {
                    purchase_writer.write(td::PurchaseTurretEvent {
                        turret_type: td::TurretType::Rapid,
                        position: player_pos,
                    });
                },
            );
            ui.add_space(6.0);

            shop_button(
                ui,
                "重型炮台",
                "150 金币\n伤害: 30 | 射速: 2s",
                gold.0 >= td::TurretType::Heavy.cost(),
                || {
                    purchase_writer.write(td::PurchaseTurretEvent {
                        turret_type: td::TurretType::Heavy,
                        position: player_pos,
                    });
                },
            );

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("操作说明")
                    .size(13.0)
                    .color(egui::Color32::GRAY),
            );
            ui.label(
                egui::RichText::new("点击建造按钮放置炮台")
                    .size(11.0)
                    .color(egui::Color32::DARK_GRAY),
            );
            ui.label(
                egui::RichText::new("炮台放在当前位置")
                    .size(11.0)
                    .color(egui::Color32::DARK_GRAY),
            );
            ui.label(
                egui::RichText::new("保护中央的水晶核心!")
                    .size(11.0)
                    .color(egui::Color32::DARK_GRAY),
            );
        });
}

fn shop_button(ui: &mut egui::Ui, name: &str, desc: &str, affordable: bool, on_click: impl FnOnce()) {
    let color = if affordable {
        egui::Color32::from_rgb(50, 150, 100)
    } else {
        egui::Color32::from_rgb(100, 60, 60)
    };

    let resp = ui.add_sized(
        egui::vec2(160.0, 60.0),
        egui::Button::new(
            egui::RichText::new(format!("{}\n{}", name, desc))
                .size(12.0),
        )
        .fill(color),
    );

    if resp.clicked() && affordable {
        on_click();
    }
}

// ═══════════════════════════════════════════
// 塔防胜利界面 (覆盖在 LevelComplete 上)
// ═══════════════════════════════════════════

fn td_victory_ui(
    mut contexts: EguiContexts,
    level: Res<LevelConfig>,
    score: Res<Score>,
    gold: Res<td::TdGold>,
    wave_state: Res<td::TdWaveState>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    // 只在 TD 关卡完成时显示
    if level.current_level != GameLevel::Level5 {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_ui(ctx, egui::Color32::from_rgba_premultiplied(0, 15, 30, 200), 140.0, |ui| {
        overlay_heading(ui, "塔防胜利!", 52.0, egui::Color32::from_rgb(50, 200, 255));
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(format!("成功抵御了所有 {} 波攻击!", wave_state.current_wave))
                .size(20.0)
                .color(egui::Color32::WHITE),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("最终分数: {} | 剩余金币: {}", score.0, gold.0))
                .size(18.0)
                .color(egui::Color32::YELLOW),
        );
        ui.add_space(40.0);

        if action_button(ui, "返回主菜单", 20.0, 200.0, 48.0) {
            main_menu_writer.write(MainMenuEvent);
        }
    });
}

fn level_complete_ui(
    mut contexts: EguiContexts,
    score: Res<Score>,
    level: Res<LevelConfig>,
    mut next_writer: MessageWriter<NextLevelEvent>,
    mut main_menu_writer: MessageWriter<MainMenuEvent>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    overlay_ui(ctx, egui::Color32::from_rgba_premultiplied(0, 20, 0, 200), 160.0, |ui| {
        overlay_heading(ui, "关卡完成!", 52.0, egui::Color32::GREEN);
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(format!("当前分数: {}", score.0))
                .size(24.0)
                .color(egui::Color32::YELLOW),
        );
        ui.add_space(40.0);

        if level.current_level.next().is_some() {
            if action_button(ui, "下一关", 22.0, 200.0, 50.0) {
                next_writer.write(NextLevelEvent);
            }
            ui.add_space(16.0);
        }
        if action_button(ui, "返回主菜单", 18.0, 200.0, 40.0) {
            main_menu_writer.write(MainMenuEvent);
        }
    });
}
