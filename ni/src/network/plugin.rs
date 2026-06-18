//! 多人聊天网络系统（TCP）
//!
//! 基于 TCP 的简单聊天协议（长度前缀 JSON），支持客户端/服务器模式。
//! 使用 `crossbeam_channel` 跨线程通信，主线程通过 Bevy 事件处理网络消息。
//! 提供 egui 聊天面板 UI。

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{Read, Write};
// TCP网络连接相关
use std::net::{TcpListener, TcpStream};
// 原子布尔值，用于安全关闭线程
use std::sync::atomic::{AtomicBool, Ordering};
// 原子引用计数，用于共享线程状态
use std::sync::Arc;
// 线程操作
use std::thread;
// 时间相关
use std::time::Duration;

// 导入游戏状态
use crate::game_state::GamePhase;

// 默认监听端口
const DEFAULT_PORT: u16 = 9999;
// 聊天记录最大保存数量
const MAX_CHAT_MESSAGES: usize = 100;

// ── 网络消息协议 ──
// 定义网络传输的数据结构与编解码规则

/// 网络传输的消息结构体（序列化后通过TCP发送）
#[derive(Serialize, Deserialize, Debug, Clone)]
struct WireMessage {
    from: String,   // 发送者名称
    text: String,   // 消息内容
}

impl WireMessage {
    /// 将消息序列化为网络字节流
    /// 格式：4字节大端长度 + JSON字符串字节
    fn to_bytes(&self) -> Vec<u8> {
        let json = serde_json::to_string(self).unwrap_or_else(|e| {
            error!("WireMessage::to_bytes: 序列化失败: {}", e);
            String::new()
        });
        let len = json.len() as u32;
        let mut bytes = Vec::with_capacity(4 + json.len());
        bytes.extend_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(json.as_bytes());
        bytes
    }
}

/// 从接收缓冲区尝试解析一条完整消息
/// 解析成功会移除缓冲区中已处理的数据，失败则保留数据等待后续接收
fn try_parse_message(buf: &mut Vec<u8>) -> Option<WireMessage> {
    // 长度头不足4字节，无法解析
    if buf.len() < 4 {
        return None;
    }
    // 读取消息长度
    let msg_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    // 消息长度异常，清空缓冲区
    if msg_len > 65536 {
        buf.clear();
        return None;
    }
    // 消息体未接收完整
    if buf.len() < 4 + msg_len {
        return None;
    }
    // 反序列化JSON消息
    let msg = serde_json::from_slice(&buf[4..4 + msg_len]).ok();
    // 移除已处理的数据
    buf.drain(..4 + msg_len);
    msg
}

// ── Bevy 消息 / 资源 ──
// 用于Bevy主线程的事件与资源定义

/// 网络接收的聊天消息事件（网络线程 → 主线程）
#[derive(Message, Clone)]
pub struct ChatMessageEvent {
    pub from: String,   // 发送者（玩家名/系统）
    pub text: String,   // 消息内容
}

/// 发送聊天消息事件（UI/游戏 → 网络线程）
#[derive(Message)]
pub struct SendChatEvent {
    pub text: String,
}

/// 单条聊天记录实体
pub struct ChatEntry {
    pub from: String,
    pub text: String,
}

/// 聊天记录资源：存储最近N条聊天消息
#[derive(Resource)]
pub struct ChatLog {
    pub messages: VecDeque<ChatEntry>,
}

impl Default for ChatLog {
    fn default() -> Self {
        Self {
            messages: VecDeque::with_capacity(MAX_CHAT_MESSAGES),
        }
    }
}

// ── 网络状态 ──
// 网络连接管理、客户端/主机状态

/// 已连接的客户端信息
struct Client {
    stream: TcpStream,  // TCP连接流
    name: String,       // 客户端昵称
    buf: Vec<u8>,       // 接收数据缓冲区
}

/// 网络状态资源：管理连接、UI输入、线程通信通道
#[derive(Resource)]
pub struct NetworkState {
    pub mode: NetworkMode,               // 当前网络模式（离线/主机/客户端）
    pub player_name: String,             // 本地玩家昵称
    pub port_input: String,              // UI端口输入框
    pub addr_input: String,              // UI地址输入框
    pub chat_input: String,              // UI聊天输入框
    incoming_rx: Option<Receiver<ChatMessageEvent>>,  // 接收网络消息通道
    outgoing_tx: Option<Sender<String>>,              // 发送消息到网络线程通道
    shutdown: Option<Arc<AtomicBool>>,                // 线程关闭信号
}

impl Default for NetworkState {
    fn default() -> Self {
        // 用系统时间生成随机默认昵称
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        Self {
            mode: NetworkMode::Offline,
            player_name: format!("Player{}", seed % 9000 + 1000),
            port_input: DEFAULT_PORT.to_string(),
            addr_input: format!("127.0.0.1:{}", DEFAULT_PORT),
            chat_input: String::new(),
            incoming_rx: None,
            outgoing_tx: None,
            shutdown: None,
        }
    }
}

/// 网络连接状态枚举
pub enum NetworkMode {
    Offline,                   // 离线
    Hosting { #[allow(dead_code)] port: u16 },     // 作为主机运行
    Client { #[allow(dead_code)] addr: String },   // 作为客户端连接
}

impl NetworkMode {
    /// 获取状态文本（用于UI显示）
    fn status_text(&self) -> &str {
        match self {
            NetworkMode::Offline => "离线",
            NetworkMode::Hosting { .. } => "主机运行中",
            NetworkMode::Client { .. } => "已连接",
        }
    }
}

impl NetworkState {
    /// 断开连接，停止网络线程，重置状态
    fn disconnect(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            shutdown.store(true, Ordering::Relaxed);
        }
        self.shutdown = None;
        self.incoming_rx = None;
        self.outgoing_tx = None;
        self.mode = NetworkMode::Offline;
    }

    /// 启动服务器（主机）模式
    fn start_host(&mut self) {
        self.disconnect();

        let port: u16 = self.port_input.parse().unwrap_or(DEFAULT_PORT);
        let (incoming_tx, incoming_rx) = crossbeam_channel::unbounded();
        let (outgoing_tx, outgoing_rx) = crossbeam_channel::unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_clone = shutdown.clone();
        let player_name = self.player_name.clone();
        // 启动独立线程运行服务器
        thread::spawn(move || {
            run_server(port, incoming_tx, outgoing_rx, shutdown_clone, player_name);
        });

        self.incoming_rx = Some(incoming_rx);
        self.outgoing_tx = Some(outgoing_tx);
        self.shutdown = Some(shutdown);
        self.mode = NetworkMode::Hosting { port };
    }

    /// 启动客户端模式，连接到主机
    fn join_host(&mut self) {
        self.disconnect();

        let addr = self.addr_input.clone();
        if addr.is_empty() {
            return;
        }
        let (incoming_tx, incoming_rx) = crossbeam_channel::unbounded();
        let (outgoing_tx, outgoing_rx) = crossbeam_channel::unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_clone = shutdown.clone();
        let player_name = self.player_name.clone();
        // 启动独立线程运行客户端
        thread::spawn(move || {
            run_client(addr, incoming_tx, outgoing_rx, shutdown_clone, player_name);
        });

        self.incoming_rx = Some(incoming_rx);
        self.outgoing_tx = Some(outgoing_tx);
        self.shutdown = Some(shutdown);
        self.mode = NetworkMode::Client {
            addr: self.addr_input.clone(),
        };
    }

    /// 发送聊天消息到网络线程
    fn send_chat(&self, text: String) {
        if let Some(ref tx) = self.outgoing_tx {
            let _ = tx.send(text);
        }
    }
}

// ── 服务器线程 ──
// 主机逻辑：监听连接、广播消息、管理客户端

fn run_server(
    port: u16,
    incoming_tx: Sender<ChatMessageEvent>,
    outgoing_rx: Receiver<String>,
    shutdown: Arc<AtomicBool>,
    host_name: String,
) {
    // 绑定TCP监听端口
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            let _ = incoming_tx.send(ChatMessageEvent {
                from: "系统".into(),
                text: format!("服务器启动失败: {}", e),
            });
            return;
        }
    };
    // 设置非阻塞模式
    listener.set_nonblocking(true).ok();

    // 发送启动成功消息
    let _ = incoming_tx.send(ChatMessageEvent {
        from: "系统".into(),
        text: format!("服务器已启动，端口 {}，等待连接...", port),
    });

    let mut clients: Vec<Client> = Vec::new();
    let mut read_buf = vec![0u8; 65536];

    // 服务器主循环
    while !shutdown.load(Ordering::Relaxed) {
        // 接受新客户端连接
        match listener.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(true).ok();
                let name = format!("Player({})", addr.port());
                // 广播新玩家加入消息
                let join_msg = WireMessage {
                    from: "系统".into(),
                    text: format!("{} 加入了聊天", name),
                };
                let wire = join_msg.to_bytes();
                for c in clients.iter_mut() {
                    let _ = c.stream.write_all(&wire);
                }
                let _ = incoming_tx.send(ChatMessageEvent {
                    from: "系统".into(),
                    text: join_msg.text,
                });
                clients.push(Client {
                    stream,
                    name,
                    buf: Vec::new(),
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        let mut disconnected_indices: Vec<usize> = Vec::new();
        let mut broadcast_queue: Vec<Vec<u8>> = Vec::new();
        let mut system_messages: Vec<ChatMessageEvent> = Vec::new();

        // 读取所有客户端消息
        for (i, client) in clients.iter_mut().enumerate() {
            match client.stream.read(&mut read_buf) {
                Ok(0) => disconnected_indices.push(i),
                Ok(n) => {
                    client.buf.extend_from_slice(&read_buf[..n]);
                    // 解析完整消息
                    while let Some(msg) = try_parse_message(&mut client.buf) {
                        // 第一条消息用于设置昵称
                        if client.name.starts_with("Player(") {
                            client.name = msg.from.clone();
                        }
                        broadcast_queue.push(msg.to_bytes());
                        system_messages.push(ChatMessageEvent {
                            from: msg.from,
                            text: msg.text,
                        });
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => disconnected_indices.push(i),
            }
        }

        // 广播消息给所有客户端
        for wire in &broadcast_queue {
            for c in clients.iter_mut() {
                let _ = c.stream.write_all(wire);
            }
        }
        // 发送消息到主线程
        for event in system_messages {
            let _ = incoming_tx.send(event);
        }

        // 处理断开的客户端
        for &i in disconnected_indices.iter().rev() {
            let name = clients[i].name.clone();
            let leave_msg = WireMessage {
                from: "系统".into(),
                text: format!("{} 离开了聊天", name),
            };
            clients.remove(i);
            let wire = leave_msg.to_bytes();
            for c in clients.iter_mut() {
                let _ = c.stream.write_all(&wire);
            }
            let _ = incoming_tx.send(ChatMessageEvent {
                from: "系统".into(),
                text: leave_msg.text,
            });
        }

        // 发送主机本地消息
        loop {
            match outgoing_rx.try_recv() {
                Ok(text) => {
                    let msg = WireMessage {
                        from: host_name.clone(),
                        text,
                    };
                    let wire = msg.to_bytes();
                    for c in clients.iter_mut() {
                        let _ = c.stream.write_all(&wire);
                    }
                    let _ = incoming_tx.send(ChatMessageEvent {
                        from: host_name.clone(),
                        text: msg.text,
                    });
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        thread::sleep(Duration::from_millis(16));
    }

    // 服务器关闭通知
    let _ = incoming_tx.send(ChatMessageEvent {
        from: "系统".into(),
        text: "服务器已关闭".into(),
    });
}

// ── 客户端线程 ──
// 客户端逻辑：连接服务器、收发消息、自动重连

fn run_client(
    server_addr: String,
    incoming_tx: Sender<ChatMessageEvent>,
    outgoing_rx: Receiver<String>,
    shutdown: Arc<AtomicBool>,
    player_name: String,
) {
    let mut stream: Option<TcpStream> = None;
    let mut buf: Vec<u8> = Vec::new();
    let mut read_buf = vec![0u8; 65536];
    let mut retry_delay = 0u32;

    // 客户端主循环
    while !shutdown.load(Ordering::Relaxed) {
        // 未连接时尝试连接服务器
        if stream.is_none() {
            match TcpStream::connect(&server_addr) {
                Ok(s) => {
                    s.set_nonblocking(true).ok();
                    let _ = incoming_tx.send(ChatMessageEvent {
                        from: "系统".into(),
                        text: format!("已连接到 {}", server_addr),
                    });
                    stream = Some(s);
                    retry_delay = 0;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                    if retry_delay == 0 {
                        let _ = incoming_tx.send(ChatMessageEvent {
                            from: "系统".into(),
                            text: format!("正在尝试连接 {}...", server_addr),
                        });
                    }
                    // 指数退避重连
                    retry_delay = (retry_delay + 1).min(60);
                    thread::sleep(Duration::from_millis(16 * retry_delay as u64));
                    continue;
                }
                Err(e) => {
                    let _ = incoming_tx.send(ChatMessageEvent {
                        from: "系统".into(),
                        text: format!("连接失败: {}", e),
                    });
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            }
        }

        let Some(s) = stream.as_mut() else { return; };

        // 读取服务器消息
        match s.read(&mut read_buf) {
            Ok(0) => {
                let _ = incoming_tx.send(ChatMessageEvent {
                    from: "系统".into(),
                    text: "与服务器断开连接".into(),
                });
                stream = None;
                buf.clear();
                continue;
            }
            Ok(n) => {
                buf.extend_from_slice(&read_buf[..n]);
                // 解析并转发消息到主线程
                while let Some(msg) = try_parse_message(&mut buf) {
                    let _ = incoming_tx.send(ChatMessageEvent {
                        from: msg.from,
                        text: msg.text,
                    });
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                let _ = incoming_tx.send(ChatMessageEvent {
                    from: "系统".into(),
                    text: format!("连接错误: {}", e),
                });
                stream = None;
                buf.clear();
                continue;
            }
        }

        // 发送本地聊天消息到服务器
        loop {
            match outgoing_rx.try_recv() {
                Ok(text) => {
                    let msg = WireMessage {
                        from: player_name.clone(),
                        text,
                    };
                    if s.write_all(&msg.to_bytes()).is_err() {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        thread::sleep(Duration::from_millis(16));
    }
}

// ── Bevy 系统 ──
// Bevy主线程系统，处理网络消息与UI事件

/// 从网络线程接收消息并添加到聊天记录
fn network_receive(state: Res<NetworkState>, mut chat_log: ResMut<ChatLog>) {
    if let Some(ref rx) = state.incoming_rx {
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    // 超过最大条数时移除最早消息
                    if chat_log.messages.len() >= MAX_CHAT_MESSAGES {
                        chat_log.messages.pop_front();
                    }
                    chat_log.messages.push_back(ChatEntry {
                        from: event.from,
                        text: event.text,
                    });
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }
}

/// 处理发送聊天事件，转发到网络线程
fn network_send(mut events: MessageReader<SendChatEvent>, state: Res<NetworkState>) {
    for ev in events.read() {
        // 忽略空消息
        if ev.text.trim().is_empty() {
            continue;
        }
        state.send_chat(ev.text.clone());
    }
}

// ── egui 聊天面板 ──
// 聊天UI界面：连接控制、消息显示、输入发送

/// 聊天UI面板，仅在聊天阶段显示
fn chat_ui_panel(
    mut state: ResMut<NetworkState>,
    chat_log: Res<ChatLog>,
    mut contexts: bevy_egui::EguiContexts,
    mut send_writer: MessageWriter<SendChatEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // 主面板
    bevy_egui::egui::CentralPanel::default()
        .frame(bevy_egui::egui::Frame {
            fill: bevy_egui::egui::Color32::from_rgba_premultiplied(10, 15, 25, 245),
            ..Default::default()
        })
        .show(ctx, |ui| {
            // 顶部：返回按钮 + 状态显示
            ui.horizontal(|ui| {
                if ui.button("← 返回主菜单").clicked() {
                    state.disconnect();
                    phase.set(GamePhase::MainMenu);
                }
                ui.with_layout(
                    bevy_egui::egui::Layout::right_to_left(bevy_egui::egui::Align::Center),
                    |ui| {
                        ui.label(
                            bevy_egui::egui::RichText::new(format!("状态: {}", state.mode.status_text()))
                                .size(14.0)
                                .color(bevy_egui::egui::Color32::GRAY),
                        );
                    },
                );
            });

            ui.separator();

            // 连接控制区：昵称、创建主机、加入服务器
            ui.horizontal(|ui| {
                ui.label("昵称:");
                ui.text_edit_singleline(&mut state.player_name);

                ui.separator();

                ui.label("端口:");
                ui.text_edit_singleline(&mut state.port_input);
                if ui.button("创建主机").clicked() {
                    state.start_host();
                }

                ui.separator();

                ui.label("地址:");
                ui.text_edit_singleline(&mut state.addr_input);
                if ui.button("加入").clicked() {
                    state.join_host();
                }

                // 连接时显示断开按钮
                if matches!(
                    state.mode,
                    NetworkMode::Hosting { .. } | NetworkMode::Client { .. }
                )
                    && ui.button("断开").clicked() {
                        state.send_chat(format!("{} 离开了聊天", state.player_name));
                        state.disconnect();
                    }
            });

            ui.separator();

            // 聊天消息滚动区域
            let available_height = ui.available_height();
            bevy_egui::egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .max_height(available_height - 40.0)
                .show(ui, |ui| {
                    for entry in &chat_log.messages {
                        // 不同角色使用不同颜色
                        let color = if entry.from == "系统" {
                            bevy_egui::egui::Color32::GRAY
                        } else if entry.from == state.player_name {
                            bevy_egui::egui::Color32::LIGHT_BLUE
                        } else {
                            bevy_egui::egui::Color32::LIGHT_GREEN
                        };
                        ui.colored_label(color, format!("{}: {}", entry.from, entry.text));
                    }
                });

            // 底部消息输入栏
            ui.with_layout(
                bevy_egui::egui::Layout::bottom_up(bevy_egui::egui::Align::Min),
                |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        let response = ui.text_edit_singleline(&mut state.chat_input);
                        let send_clicked = ui.button("发送").clicked();
                        let enter_pressed = response.has_focus()
                            && ui.input(|i| i.key_pressed(bevy_egui::egui::Key::Enter));
                        // 发送消息（点击按钮/按回车）
                        if (send_clicked || enter_pressed) && !state.chat_input.trim().is_empty() {
                            send_writer.write(SendChatEvent {
                                text: state.chat_input.trim().to_string(),
                            });
                            state.chat_input.clear();
                            response.request_focus();
                        }
                    });
                },
            );
        });
}

// ── 插件 ──
// 网络聊天插件入口

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ChatMessageEvent>()
            .add_message::<SendChatEvent>()
            .init_resource::<ChatLog>()
            .init_resource::<NetworkState>()
            // 网络消息收发系统
            .add_systems(Update, (network_receive, network_send))
            // 聊天UI仅在聊天阶段运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                chat_ui_panel.run_if(in_state(GamePhase::MultiplayerChat)),
            );
    }
}