use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::game_state::GamePhase;

const DEFAULT_PORT: u16 = 9999;
const MAX_CHAT_MESSAGES: usize = 100;

// ── 网络消息协议 ──

/// 网络传输格式（4字节大端长度 + JSON）
#[derive(Serialize, Deserialize, Debug, Clone)]
struct WireMessage {
    from: String,
    text: String,
}

impl WireMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let json = serde_json::to_string(self).unwrap();
        let len = json.len() as u32;
        let mut bytes = Vec::with_capacity(4 + json.len());
        bytes.extend_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(json.as_bytes());
        bytes
    }
}

/// 从字节缓冲区中尝试解析一个完整消息
fn try_parse_message(buf: &mut Vec<u8>) -> Option<WireMessage> {
    if buf.len() < 4 {
        return None;
    }
    let msg_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if msg_len > 65536 {
        buf.clear();
        return None;
    }
    if buf.len() < 4 + msg_len {
        return None;
    }
    let msg = serde_json::from_slice(&buf[4..4 + msg_len]).ok();
    buf.drain(..4 + msg_len);
    msg
}

// ── Bevy 消息/资源 ──

/// 接收到的聊天消息
#[derive(Message, Clone)]
pub struct ChatMessageEvent {
    pub from: String,
    pub text: String,
}

/// 用户发送聊天消息
#[derive(Message)]
pub struct SendChatEvent {
    pub text: String,
}

pub struct ChatEntry {
    pub from: String,
    pub text: String,
}

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

/// 客户端连接信息
struct Client {
    stream: TcpStream,
    name: String,
    buf: Vec<u8>,
}

#[derive(Resource)]
pub struct NetworkState {
    pub mode: NetworkMode,
    pub player_name: String,
    pub port_input: String,
    pub addr_input: String,
    pub chat_input: String,
    incoming_rx: Option<Receiver<ChatMessageEvent>>,
    outgoing_tx: Option<Sender<String>>,
    shutdown: Option<Arc<AtomicBool>>,
}

impl Default for NetworkState {
    fn default() -> Self {
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

pub enum NetworkMode {
    Offline,
    Hosting {
        #[allow(dead_code)]
        port: u16,
    },
    Client {
        #[allow(dead_code)]
        addr: String,
    },
}

impl NetworkMode {
    fn status_text(&self) -> &str {
        match self {
            NetworkMode::Offline => "离线",
            NetworkMode::Hosting { .. } => "主机运行中",
            NetworkMode::Client { .. } => "已连接",
        }
    }
}

impl NetworkState {
    /// 停止当前网络连接
    fn disconnect(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            shutdown.store(true, Ordering::Relaxed);
        }
        self.shutdown = None;
        self.incoming_rx = None;
        self.outgoing_tx = None;
        self.mode = NetworkMode::Offline;
    }

    /// 启动主机（Listen Server）
    fn start_host(&mut self) {
        self.disconnect();

        let port: u16 = self.port_input.parse().unwrap_or(DEFAULT_PORT);
        let (incoming_tx, incoming_rx) = crossbeam_channel::unbounded();
        let (outgoing_tx, outgoing_rx) = crossbeam_channel::unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_clone = shutdown.clone();
        let player_name = self.player_name.clone();
        thread::spawn(move || {
            run_server(port, incoming_tx, outgoing_rx, shutdown_clone, player_name);
        });

        self.incoming_rx = Some(incoming_rx);
        self.outgoing_tx = Some(outgoing_tx);
        self.shutdown = Some(shutdown);
        self.mode = NetworkMode::Hosting { port };
    }

    /// 加入远程主机
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

    fn send_chat(&self, text: String) {
        if let Some(ref tx) = self.outgoing_tx {
            let _ = tx.send(text);
        }
    }
}

// ── 服务器线程 ──

fn run_server(
    port: u16,
    incoming_tx: Sender<ChatMessageEvent>,
    outgoing_rx: Receiver<String>,
    shutdown: Arc<AtomicBool>,
    host_name: String,
) {
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
    listener.set_nonblocking(true).ok();

    let _ = incoming_tx.send(ChatMessageEvent {
        from: "系统".into(),
        text: format!("服务器已启动，端口 {}，等待连接...", port),
    });

    let mut clients: Vec<Client> = Vec::new();
    let mut read_buf = vec![0u8; 65536];

    while !shutdown.load(Ordering::Relaxed) {
        // 接受新连接
        match listener.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(true).ok();
                let name = format!("Player({})", addr.port());
                // 广播加入消息
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

        // 读取客户端消息（先收集，再广播，避免借用冲突）
        let mut disconnected_indices: Vec<usize> = Vec::new();
        let mut broadcast_queue: Vec<Vec<u8>> = Vec::new();
        let mut system_messages: Vec<ChatMessageEvent> = Vec::new();

        for (i, client) in clients.iter_mut().enumerate() {
            match client.stream.read(&mut read_buf) {
                Ok(0) => disconnected_indices.push(i),
                Ok(n) => {
                    client.buf.extend_from_slice(&read_buf[..n]);
                    while let Some(msg) = try_parse_message(&mut client.buf) {
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

        // 广播收集到的消息
        for wire in &broadcast_queue {
            for c in clients.iter_mut() {
                let _ = c.stream.write_all(wire);
            }
        }
        for event in system_messages {
            let _ = incoming_tx.send(event);
        }

        // 处理断开连接（倒序移除）
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

        // 处理主机消息
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

    let _ = incoming_tx.send(ChatMessageEvent {
        from: "系统".into(),
        text: "服务器已关闭".into(),
    });
}

// ── 客户端线程 ──

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

    while !shutdown.load(Ordering::Relaxed) {
        // 尝试连接
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

        let s = stream.as_mut().unwrap();

        // 读取
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

        // 发送
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

/// 从网络线程接收消息并存入聊天记录
fn network_receive(state: Res<NetworkState>, mut chat_log: ResMut<ChatLog>) {
    if let Some(ref rx) = state.incoming_rx {
        loop {
            match rx.try_recv() {
                Ok(event) => {
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

/// 发送聊天消息到网络线程
fn network_send(mut events: MessageReader<SendChatEvent>, state: Res<NetworkState>) {
    for ev in events.read() {
        if ev.text.trim().is_empty() {
            continue;
        }
        state.send_chat(ev.text.clone());
    }
}

// ── egui 聊天面板 ──

fn chat_ui_panel(
    mut state: ResMut<NetworkState>,
    chat_log: Res<ChatLog>,
    mut contexts: bevy_egui::EguiContexts,
    mut send_writer: MessageWriter<SendChatEvent>,
    mut phase: ResMut<NextState<GamePhase>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // 全屏聊天界面，使用类似主菜单的覆盖层风格
    bevy_egui::egui::CentralPanel::default()
        .frame(bevy_egui::egui::Frame {
            fill: bevy_egui::egui::Color32::from_rgba_premultiplied(10, 15, 25, 245),
            ..Default::default()
        })
        .show(ctx, |ui| {
            // 顶部栏：标题 + 返回按钮
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

            // 连接控制区
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

                if matches!(
                    state.mode,
                    NetworkMode::Hosting { .. } | NetworkMode::Client { .. }
                ) {
                    if ui.button("断开").clicked() {
                        state.send_chat(format!("{} 离开了聊天", state.player_name));
                        state.disconnect();
                    }
                }
            });

            ui.separator();

            // 聊天消息区域
            let available_height = ui.available_height();
            bevy_egui::egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .max_height(available_height - 40.0)
                .show(ui, |ui| {
                    for entry in &chat_log.messages {
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

            // 输入行固定在底部
            ui.with_layout(
                bevy_egui::egui::Layout::bottom_up(bevy_egui::egui::Align::Min),
                |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        let response = ui.text_edit_singleline(&mut state.chat_input);
                        let send_clicked = ui.button("发送").clicked();
                        let enter_pressed = response.has_focus()
                            && ui.input(|i| i.key_pressed(bevy_egui::egui::Key::Enter));
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

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ChatMessageEvent>()
            .add_message::<SendChatEvent>()
            .init_resource::<ChatLog>()
            .init_resource::<NetworkState>()
            .add_systems(Update, (network_receive, network_send))
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                chat_ui_panel.run_if(in_state(GamePhase::MultiplayerChat)),
            );
    }
}
