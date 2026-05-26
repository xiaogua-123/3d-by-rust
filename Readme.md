# NI - 从零开始的 3D 冒险

---

## 项目介绍

**NI** 是一款使用 **Rust** 语言与 **Bevy 0.18** 引擎打造的 3D 冒险游戏，融合了多种玩法机制和自定义渲染效果。

### 核心技术

| 分类 | 技术 | 用途 |
|------|------|------|
| 引擎 | Bevy 0.18 | ECS 游戏引擎，负责渲染、物理、场景管理 |
| 语言 | Rust (edition 2024) | 高性能、内存安全的系统编程语言 |
| UI | egui (bevy_egui 0.39) | 主菜单、HUD、暂停菜单等界面 |
| 调试 | bevy-inspector-egui 0.36 | World Inspector 面板，实时查看实体和组件 |
| 序列化 | serde / serde_json / RON | 关卡数据、配置文件、网络消息的序列化 |
| 并发 | crossbeam-channel | 多线程消息传递 |
| 粒子 | bevy_hanabi 0.18 | GPU 粒子特效系统 |
| 光照 | bevy_solari | Solari 光照/阴影系统 |
| 随机 | rand 0.8 | 敌人生成、掉落等随机逻辑 |

### 功能模块

| 模块 | 说明 |
|------|------|
| 3D 角色控制 | WASD 移动、空格跳跃、鼠标视角 |
| 关卡系统 | 4 个关卡，难度递增（草原→庭院→废墟→城市） |
| 收集系统 | 金色光球收集，全部收集后通关 |
| 敌人 AI | 巡逻敌人、追击敌人，触碰扣血 |
| 生命/战斗系统 | 生命值、伤害、游戏结束判定 |
| 塔防模式 (`td/`) | 炮塔放置、敌人波次、弹道系统 |
| 卡通渲染 (`toon/`) | 自定义描边、材质、渐变纹理 |
| 粒子特效 | 收集特效、攻击特效、环境粒子 |
| NPC 系统 | NPC 交互、对话系统 |
| 动画系统 | 实体动画控制 |
| 库存系统 | 物品收集与管理 |
| 网络模块 | 网络通信支持 |
| 音频系统 | WAV 音效播放（跳跃、收集、受伤等） |
| 碰撞检测 | AABB 碰撞检测 |

### 构建与运行

```bash
cd ni
cargo run              # debug 模式（含 World Inspector）
cargo run --release     # release 模式（LTO + strip 优化）
```

---

## 中文

### 游戏介绍
一个基于 Bevy 引擎的 3D 冒险游戏。在 3D 场景中移动、收集物品、躲避敌人，完成所有关卡！

### 操作说明

| 操作 | 按键 |
|------|------|
| 移动 | **W/A/S/D** 或 **方向键** |
| 跳跃 | **空格键** |
| 视角 | **鼠标移动** |
| 暂停 | **ESC** |
| 调试切换关卡 | **1 / 2 / 3 / 4** |

### 游戏目标
- 收集关卡中的所有**金色光球**来过关
- 躲避**红色敌人**，触碰会扣血
- 共有 **4 个关卡**，难度递增
- 生命值归零则游戏结束

### 游戏界面

**主菜单**
- **开始游戏**：开始新游戏
- **退出**：退出游戏

**HUD（游戏界面顶部）**
- 显示当前关卡名称
- 显示生命值（❤）
- 显示分数
- 显示收集进度

**暂停菜单（按 ESC）**
- **继续游戏**：回到游戏
- **返回主菜单**：回到主菜单

**关卡完成**
- 收集完所有光球后显示
- **下一关**：进入下一关
- **返回主菜单**：回到主菜单

**游戏结束**
- 生命值归零时显示
- **重新开始**：从第 1 关重新开始
- **返回主菜单**：回到主菜单

### 关卡介绍

| 关卡 | 名称 | 场景 | 收集品 | 敌人 |
|------|------|------|--------|------|
| 1 | 绿色草原 | 绿色地面 + 浮动平台 | 3 | 1 个巡逻敌人 |
| 2 | 蓝色庭院 | 蓝色地面 + GLB 装饰物 | 5 | 2 个巡逻敌人 |
| 3 | 黑暗废墟 | 深色地面 + 围墙迷宫 | 7 | 2 个快速敌人 |
| 4 | 城市 | 3D 城市模型场景 | 5 | 2 个巡逻敌人 |

### 运行游戏
```bash
cd ni
cargo run
```

### 技术栈
- **引擎**: Bevy 0.18（ECS 架构，3D 渲染，物理）
- **语言**: Rust (edition 2024)
- **UI**: egui (bevy_egui 0.39) — 菜单、HUD、面板
- **调试**: bevy-inspector-egui — 实体/组件实时查看
- **序列化**: serde + serde_json + RON — 关卡数据与配置
- **并发**: crossbeam-channel — 多线程通信
- **粒子**: bevy_hanabi 0.18 — GPU 粒子特效
- **光照**: bevy_solari — 光影系统
- **3D 模型**: GLB 格式
- **音频**: WAV 格式

### 调试模式
在 debug 模式下运行时，会额外显示 World Inspector 面板，方便调试实体和组件。

---

## English

### About
A 3D adventure game built with the Bevy engine. Explore 3D environments, collect items, dodge enemies, and clear all levels!

### Controls

| Action | Key |
|--------|-----|
| Move | **W/A/S/D** or **Arrow Keys** |
| Jump | **Space** |
| Look | **Mouse** |
| Pause | **ESC** |
| Debug Switch Level | **1 / 2 / 3 / 4** |

### Objective
- Collect all **golden orbs** in each level to advance
- Avoid **red enemies** — contact costs health
- **4 levels** with increasing difficulty
- Game over when health reaches zero

### UI

**Main Menu**
- **Start Game**: Begin a new game
- **Quit**: Exit the game

**HUD (top of screen)**
- Current level name
- Health (❤)
- Score
- Collection progress

**Pause Menu (ESC)**
- **Resume**: Return to game
- **Main Menu**: Return to main menu

**Level Complete**
- Shown after collecting all orbs
- **Next Level**: Proceed to next level
- **Main Menu**: Return to main menu

**Game Over**
- Shown when health reaches zero
- **Restart**: Restart from level 1
- **Main Menu**: Return to main menu

### Levels

| Level | Name | Layout | Collectibles | Enemies |
|-------|------|--------|-------------|---------|
| 1 | Green Meadow | Green ground + floating platforms | 3 | 1 patrol |
| 2 | Blue Courtyard | Blue ground + GLB decorations | 5 | 2 patrols |
| 3 | Dark Ruins | Dark ground + maze walls | 7 | 2 fast patrols |
| 4 | City | 3D city model scene | 5 | 2 patrols |

### Running
```bash
cd ni
cargo run
```

### Tech Stack
- **Engine**: Bevy 0.18 (ECS architecture, 3D rendering, physics)
- **Language**: Rust (edition 2024)
- **UI**: egui (bevy_egui 0.39) — menus, HUD, panels
- **Debug**: bevy-inspector-egui — real-time entity/component inspection
- **Serialization**: serde + serde_json + RON — level data & config
- **Concurrency**: crossbeam-channel — multi-threaded messaging
- **Particles**: bevy_hanabi 0.18 — GPU particle effects
- **Lighting**: bevy_solari — light & shadow system
- **3D Models**: GLB format
- **Audio**: WAV format

### Debug Mode
Running in debug mode also shows a World Inspector panel for inspecting entities and components.

---

## 日本語

### ゲーム紹介
Bevy エンジンで制作した 3D アドベンチャーゲーム。3D 空間を探索し、アイテムを集め、敵を避けて全ステージをクリアしよう！

### 操作

| 操作 | キー |
|------|------|
| 移動 | **W/A/S/D** または **矢印キー** |
| ジャンプ | **スペース** |
| 視点 | **マウス** |
| 一時停止 | **ESC** |
| デバッグ切替 | **1 / 2 / 3 / 4** |

### 目標
- ステージ内の**金色のオーブ**を全て集めるとクリア
- **赤い敵**を避ける（接触でダメージ）
- 全 **4 ステージ**、難易度が上がる
- ライフが 0 になるとゲームオーバー

### UI

**メインメニュー**
- **ゲーム開始**: 新規ゲーム開始
- **終了**: ゲーム終了

**HUD（画面表示）**
- 現在のステージ名
- ライフ（❤）
- スコア
- 収集進捗

**ポーズメニュー（ESC）**
- **続行**: ゲームに戻る
- **メインメニュー**: メインメニューに戻る

**ステージクリア**
- オーブを全部集めると表示
- **次へ**: 次のステージへ
- **メインメニュー**: メインメニューに戻る

**ゲームオーバー**
- ライフ 0 で表示
- **リスタート**: ステージ 1 から再開
- **メインメニュー**: メインメニューに戻る

### ステージ

| 番号 | 名称 | 構成 | 収集物 | 敵 |
|------|------|------|--------|-----|
| 1 | 緑の草原 | 緑地面 + 浮遊台 | 3 | 1 体 |
| 2 | 青の中庭 | 青地面 + GLB 装飾 | 5 | 2 体 |
| 3 | 闇の廃墟 | 暗色地面 + 迷路壁 | 7 | 2 体（高速） |
| 4 | 都市 | 3D 都市モデル | 5 | 2 体 |

### 実行方法
```bash
cd ni
cargo run
```

### 技術構成
- **エンジン**: Bevy 0.18（ECS アーキテクチャ、3D 描画、物理）
- **言語**: Rust (edition 2024)
- **UI**: egui (bevy_egui 0.39) — メニュー、HUD、パネル
- **デバッグ**: bevy-inspector-egui — エンティティ/コンポーネントのリアルタイム表示
- **シリアライズ**: serde + serde_json + RON — レベルデータと設定
- **並行性**: crossbeam-channel — マルチスレッド通信
- **パーティクル**: bevy_hanabi 0.18 — GPU パーティクルエフェクト
- **ライティング**: bevy_solari — 光源と影のシステム
- **3D モデル**: GLB 形式
- **音声**: WAV 形式

### デバッグモード
debug モード実行時、World Inspector パネルが表示され、エンティティやコンポーネントを確認できます。

---

## 한국어

### 게임 소개
Bevy 엔진으로 제작한 3D 어드벤처 게임. 3D 공간을 탐험하고, 아이템을 모으고, 적을 피해 모든 스테이지를 클리어하세요!

### 조작

| 동작 | 키 |
|------|-----|
| 이동 | **W/A/S/D** 또는 **방향키** |
| 점프 | **스페이스** |
| 시점 | **마우스** |
| 일시정지 | **ESC** |
| 디버그 전환 | **1 / 2 / 3 / 4** |

### 목표
- 스테이지의 모든 **황금 오브**를 수집하면 클리어
- **빨간 적**을 피하세요 (접촉 시 체력 감소)
- 총 **4개 스테이지**, 난이도 상승
- 체력이 0이면 게임 오버

### UI

**메인 메뉴**
- **게임 시작**: 새 게임 시작
- **종료**: 게임 종료

**HUD (화면 상단)**
- 현재 스테이지 이름
- 체력 (❤)
- 점수
- 수집 진행도

**일시정지 메뉴 (ESC)**
- **계속하기**: 게임으로 복귀
- **메인 메뉴**: 메인 메뉴로 이동

**스테이지 클리어**
- 모든 오브 수집 시 표시
- **다음**: 다음 스테이지로
- **메인 메뉴**: 메인 메뉴로 이동

**게임 오버**
- 체력 0 시 표시
- **재시작**: 스테이지 1부터 재시작
- **메인 메뉴**: 메인 메뉴로 이동

### 스테이지

| 번호 | 이름 | 구성 | 수집품 | 적 |
|------|------|------|--------|-----|
| 1 | 초록 초원 | 녹색 바닥 + 부유 플랫폼 | 3 | 1명 순찰 |
| 2 | 파란 정원 | 파란 바닥 + GLB 장식 | 5 | 2명 순찰 |
| 3 | 어둠의 폐허 | 어두운 바닥 + 미로 벽 | 7 | 2명 고속 순찰 |
| 4 | 도시 | 3D 도시 모델 | 5 | 2명 순찰 |

### 실행
```bash
cd ni
cargo run
```

### 기술 스택
- **엔진**: Bevy 0.18 (ECS 아키텍처, 3D 렌더링, 물리)
- **언어**: Rust (edition 2024)
- **UI**: egui (bevy_egui 0.39) — 메뉴, HUD, 패널
- **디버그**: bevy-inspector-egui — 엔티티/컴포넌트 실시간 검사
- **직렬화**: serde + serde_json + RON — 레벨 데이터 및 설정
- **동시성**: crossbeam-channel — 멀티스레드 메시징
- **파티클**: bevy_hanabi 0.18 — GPU 파티클 이펙트
- **라이팅**: bevy_solari — 조명 및 그림자 시스템
- **3D 모델**: GLB 형식
- **오디오**: WAV 형식

### 디버그 모드
debug 모드 실행 시 World Inspector 패널이 표시되어 엔티티와 컴포넌트를 확인할 수 있습니다.

---

## Русский

### Об игре
3D-приключение на движке Bevy. Исследуйте трёхмерные сцены, собирайте предметы, уворачивайтесь от врагов и проходите все уровни!

### Управление

| Действие | Клавиша |
|----------|---------|
| Движение | **W/A/S/D** или **стрелки** |
| Прыжок | **Пробел** |
| Обзор | **Мышь** |
| Пауза | **ESC** |
| Смена уровня (отладка) | **1 / 2 / 3 / 4** |

### Цель
- Соберите все **золотые сферы** на уровне для прохождения
- Избегайте **красных врагов** — касание отнимает здоровье
- **4 уровня** с возрастающей сложностью
- Здоровье на нуле — игра окончена

### Интерфейс

**Главное меню**
- **Начать игру**: Новая игра
- **Выход**: Выйти из игры

**HUD (вверху экрана)**
- Название текущего уровня
- Здоровье (❤)
- Очки
- Прогресс сбора

**Меню паузы (ESC)**
- **Продолжить**: Вернуться к игре
- **Главное меню**: Выйти в меню

**Уровень пройден**
- Показывается после сбора всех сфер
- **Далее**: Следующий уровень
- **Главное меню**: Выйти в меню

**Игра окончена**
- Показывается при нулевом здоровье
- **Заново**: Начать с уровня 1
- **Главное меню**: Выйти в меню

### Уровни

| № | Название | Описание | Предметы | Враги |
|----|----------|----------|----------|-------|
| 1 | Зелёный луг | Зелёная земля + платформы | 3 | 1 патрульный |
| 2 | Синий двор | Синяя земля + GLB декор | 5 | 2 патрульных |
| 3 | Тёмные руины | Тёмная земля + лабиринт | 7 | 2 быстрых |
| 4 | Город | 3D-модель города | 5 | 2 патрульных |

### Запуск
```bash
cd ni
cargo run
```

### Технологии
- **Движок**: Bevy 0.18 (ECS-архитектура, 3D-рендеринг, физика)
- **Язык**: Rust (edition 2024)
- **UI**: egui (bevy_egui 0.39) — меню, HUD, панели
- **Отладка**: bevy-inspector-egui — просмотр сущностей/компонентов в реальном времени
- **Сериализация**: serde + serde_json + RON — данные уровней и конфигурация
- **Многопоточность**: crossbeam-channel — обмен сообщениями между потоками
- **Частицы**: bevy_hanabi 0.18 — GPU-эффекты частиц
- **Освещение**: bevy_solari — система света и теней
- **3D-модели**: формат GLB
- **Аудио**: формат WAV

### Режим отладки
В debug-режиме отображается панель World Inspector для проверки сущностей и компонентов.

---

## Français

### Présentation
Un jeu d'aventure 3D construit avec le moteur Bevy. Explorez des environnements 3D, collectez des objets, évitez les ennemis et terminez tous les niveaux !

### Commandes

| Action | Touche |
|--------|--------|
| Se déplacer | **W/A/S/D** ou **flèches** |
| Sauter | **Espace** |
| Regarder | **Souris** |
| Pause | **Échap** |
| Changer niveau (debug) | **1 / 2 / 3 / 4** |

### Objectif
- Ramassez tous les **orbes dorés** du niveau pour passer au suivant
- Évitez les **ennemis rouges** — le contact réduit la vie
- **4 niveaux** de difficulté croissante
- Partie terminée quand la vie tombe à zéro

### Interface

**Menu Principal**
- **Nouvelle partie** : Commencer une nouvelle partie
- **Quitter** : Quitter le jeu

**HUD (en haut de l'écran)**
- Nom du niveau actuel
- Vie (❤)
- Score
- Progression de collecte

**Menu Pause (Échap)**
- **Reprendre** : Retour au jeu
- **Menu principal** : Retour au menu

**Niveau terminé**
- Affiché après avoir collecté tous les orbes
- **Suivant** : Niveau suivant
- **Menu principal** : Retour au menu

**Game Over**
- Affiché quand la vie atteint zéro
- **Recommencer** : Reprendre du niveau 1
- **Menu principal** : Retour au menu

### Niveaux

| N° | Nom | Description | Objets | Ennemis |
|----|-----|-------------|--------|---------|
| 1 | Prairie verte | Sol vert + plateformes | 3 | 1 patrouilleur |
| 2 | Cour bleue | Sol bleu + décors GLB | 5 | 2 patrouilleurs |
| 3 | Ruines sombres | Sol sombre + labyrinthe | 7 | 2 rapides |
| 4 | Ville | Modèle 3D de ville | 5 | 2 patrouilleurs |

### Lancer le jeu
```bash
cd ni
cargo run
```

### Stack Technique
- **Moteur** : Bevy 0.18 (architecture ECS, rendu 3D, physique)
- **Langage** : Rust (edition 2024)
- **UI** : egui (bevy_egui 0.39) — menus, HUD, panneaux
- **Débogage** : bevy-inspector-egui — inspection en temps réel des entités/composants
- **Sérialisation** : serde + serde_json + RON — données de niveaux et configuration
- **Concurrence** : crossbeam-channel — messagerie multi-thread
- **Particules** : bevy_hanabi 0.18 — effets de particules GPU
- **Éclairage** : bevy_solari — système de lumière et ombres
- **Modèles 3D** : format GLB
- **Audio** : format WAV

### Mode Debug
En mode debug, un panneau World Inspector s'affiche pour inspecter les entités et composants.

---

## Deutsch

### Über das Spiel
Ein 3D-Abenteuerspiel mit der Bevy-Engine. Erkunde 3D-Umgebungen, sammle Gegenstände, weiche Feinden aus und meistere alle Level!

### Steuerung

| Aktion | Taste |
|--------|-------|
| Bewegen | **W/A/S/D** oder **Pfeiltasten** |
| Springen | **Leertaste** |
| Umschauen | **Maus** |
| Pause | **ESC** |
| Level wechseln (Debug) | **1 / 2 / 3 / 4** |

### Ziel
- Sammle alle **goldenen Kugeln** im Level ein, um weiterzukommen
- Weiche **roten Feinden** aus — Kontakt kostet Leben
- **4 Level** mit steigendem Schwierigkeitsgrad
- Spiel vorbei, wenn die Gesundheit Null erreicht

### Benutzeroberfläche

**Hauptmenü**
- **Spiel starten**: Neues Spiel beginnen
- **Beenden**: Spiel beenden

**HUD (oben am Bildschirm)**
- Aktueller Levelname
- Gesundheit (❤)
- Punktzahl
- Sammelfortschritt

**Pausenmenü (ESC)**
- **Fortsetzen**: Zurück zum Spiel
- **Hauptmenü**: Zurück zum Hauptmenü

**Level geschafft**
- Wird nach dem Einsammeln aller Kugeln angezeigt
- **Weiter**: Nächstes Level
- **Hauptmenü**: Zurück zum Hauptmenü

**Game Over**
- Wird angezeigt, wenn die Gesundheit Null erreicht
- **Neustart**: Von Level 1 neu starten
- **Hauptmenü**: Zurück zum Hauptmenü

### Level

| Nr. | Name | Aufbau | Items | Feinde |
|-----|------|--------|-------|--------|
| 1 | Grüne Wiese | Grüner Boden + Plattformen | 3 | 1 Wächter |
| 2 | Blauer Hof | Blauer Boden + GLB-Deko | 5 | 2 Wächter |
| 3 | Dunkle Ruinen | Dunkler Boden + Labyrinth | 7 | 2 schnelle |
| 4 | Stadt | 3D-Stadtmodell | 5 | 2 Wächter |

### Ausführen
```bash
cd ni
cargo run
```

### Tech-Stack
- **Engine**: Bevy 0.18 (ECS-Architektur, 3D-Rendering, Physik)
- **Sprache**: Rust (edition 2024)
- **UI**: egui (bevy_egui 0.39) — Menüs, HUD, Panels
- **Debug**: bevy-inspector-egui — Echtzeit-Ansicht von Entitäten/Komponenten
- **Serialisierung**: serde + serde_json + RON — Leveldaten & Konfiguration
- **Nebenläufigkeit**: crossbeam-channel — Thread-übergreifende Kommunikation
- **Partikel**: bevy_hanabi 0.18 — GPU-Partikeleffekte
- **Beleuchtung**: bevy_solari — Licht- und Schattensystem
- **3D-Modelle**: GLB-Format
- **Audio**: WAV-Format

### Debug-Modus
Im Debug-Modus wird ein World-Inspektor-Panel angezeigt, mit dem Entitäten und Komponenten untersucht werden können.
