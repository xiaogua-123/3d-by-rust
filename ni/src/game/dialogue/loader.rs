//! 对话系统 — 从 RON 文件加载对话和任务

use bevy::prelude::*;
use ron::de::from_reader;
use std::fs;

use crate::game::dialogue::quest::{QuestBank, QuestDef, SubgoalDef};
use crate::game::dialogue::types::*;

pub fn load_dialogues(mut bank: ResMut<DialogueBank>) {
    let dir = "assets/dialogue";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron") {
                match fs::File::open(&path) {
                    Ok(file) => match from_reader::<_, DialogueConversation>(file) {
                        Ok(conv) => {
                            bank.conversations.insert(conv.id.clone(), conv);
                        }
                        Err(e) => error!("解析对话文件失败 {:?}: {}", path, e),
                    },
                    Err(e) => error!("打开对话文件失败 {:?}: {}", path, e),
                }
            }
        }
    } else {
        let _ = fs::create_dir_all(dir);
        error!("对话目录不存在，已创建 assets/dialogue/，请放入 .ron 对话文件");
    }
}

pub fn load_quests(mut bank: ResMut<QuestBank>) {
    let quests = vec![
        QuestDef {
            id: "investigate_forest".into(),
            name: "调查森林".into(),
            description: "东边的森林出现了怪物，前往调查。".into(),
            subgoals: vec![
                SubgoalDef {
                    description: "与村长交谈".into(),
                    completion_flag: None,
                },
                SubgoalDef {
                    description: "前往东翼走廊".into(),
                    completion_flag: Some("visited_east_wing".into()),
                },
                SubgoalDef {
                    description: "调查森林中的异常".into(),
                    completion_flag: None,
                },
            ],
        },
    ];
    for q in quests {
        bank.quests.insert(q.id.clone(), q);
    }
}
