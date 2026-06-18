//! 对话系统 — 条件检查与效果执行

use bevy::prelude::*;

use crate::dialogue::QuestTracker;
use crate::inventory::Inventory;
use crate::game::dialogue::types::*;

impl DialogueCondition {
    pub fn check(&self, quests: &QuestTracker, inventory: &Inventory) -> bool {
        match self {
            DialogueCondition::HasItem(id) => inventory.has(id),
            DialogueCondition::NoItem(id) => !inventory.has(id),
            DialogueCondition::QuestComplete(id) => quests.completed_quests.contains(id),
            DialogueCondition::QuestActive(id) => quests.active_quests.contains(id),
            DialogueCondition::Flag(f) => quests.flags.contains(f),
            DialogueCondition::HasVisitedZone(id) => {
                quests.flags.contains(&format!("visited_{}", id))
            }
        }
    }
}

pub fn apply_effects(
    effects: &[DialogueEffect],
    quests: &mut QuestTracker,
) -> Vec<PendingEffect> {
    let mut pending = Vec::new();
    for effect in effects {
        match effect {
            DialogueEffect::StartQuest(id) => {
                if !quests.active_quests.contains(id) {
                    quests.active_quests.push(id.clone());
                }
            }
            DialogueEffect::CompleteQuest(id) => {
                quests.active_quests.retain(|q| q != id);
                if !quests.completed_quests.contains(id) {
                    quests.completed_quests.push(id.clone());
                }
            }
            DialogueEffect::SetFlag(f) => {
                if !quests.flags.contains(f) {
                    quests.flags.push(f.clone());
                }
            }
            DialogueEffect::GiveItem(id, amount) => {
                pending.push(PendingEffect::GiveItem(id.clone(), *amount));
            }
            DialogueEffect::RemoveItem(id, amount) => {
                pending.push(PendingEffect::RemoveItem(id.clone(), *amount));
            }
            DialogueEffect::StartPuzzle(id) => info!("对话效果: 启动谜题 {}", id),
            DialogueEffect::UnlockDoor(id) => info!("对话效果: 解锁门 {}", id),
            DialogueEffect::PlayCutscene(id) => info!("对话效果: 播放过场 {}", id),
        }
    }
    pending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_quest_adds_to_active() {
        let mut tracker = QuestTracker::default();
        let effects = vec![DialogueEffect::StartQuest("test_quest".into())];
        let pending = apply_effects(&effects, &mut tracker);
        assert!(tracker.active_quests.contains(&"test_quest".into()));
        assert!(!tracker.completed_quests.contains(&"test_quest".into()));
        assert!(pending.is_empty());
    }

    #[test]
    fn start_quest_no_duplicate() {
        let mut tracker = QuestTracker::default();
        tracker.active_quests.push("test_quest".into());
        let effects = vec![DialogueEffect::StartQuest("test_quest".into())];
        apply_effects(&effects, &mut tracker);
        assert_eq!(tracker.active_quests.iter().filter(|q| *q == "test_quest").count(), 1);
    }

    #[test]
    fn complete_quest_moves_from_active_to_completed() {
        let mut tracker = QuestTracker::default();
        tracker.active_quests.push("test_quest".into());
        let effects = vec![DialogueEffect::CompleteQuest("test_quest".into())];
        apply_effects(&effects, &mut tracker);
        assert!(!tracker.active_quests.contains(&"test_quest".into()));
        assert!(tracker.completed_quests.contains(&"test_quest".into()));
    }

    #[test]
    fn give_item_creates_pending_effect() {
        let mut tracker = QuestTracker::default();
        let effects = vec![DialogueEffect::GiveItem("sword".into(), 1)];
        let pending = apply_effects(&effects, &mut tracker);
        assert_eq!(pending.len(), 1);
        match &pending[0] {
            PendingEffect::GiveItem(id, amount) => {
                assert_eq!(id, "sword");
                assert_eq!(*amount, 1);
            }
            _ => panic!("预期 GiveItem"),
        }
    }

    #[test]
    fn remove_item_creates_pending_effect() {
        let mut tracker = QuestTracker::default();
        let effects = vec![DialogueEffect::RemoveItem("potion".into(), 2)];
        let pending = apply_effects(&effects, &mut tracker);
        assert_eq!(pending.len(), 1);
        match &pending[0] {
            PendingEffect::RemoveItem(id, amount) => {
                assert_eq!(id, "potion");
                assert_eq!(*amount, 2);
            }
            _ => panic!("预期 RemoveItem"),
        }
    }

    #[test]
    fn set_flag_adds_unique() {
        let mut tracker = QuestTracker::default();
        let effects = vec![DialogueEffect::SetFlag("visited_town".into())];
        apply_effects(&effects, &mut tracker);
        assert!(tracker.flags.contains(&"visited_town".into()));
        // 重复设置不应添加重复 flag
        apply_effects(&effects, &mut tracker);
        assert_eq!(tracker.flags.iter().filter(|f| *f == "visited_town").count(), 1);
    }

    #[test]
    fn multiple_effects_in_order() {
        let mut tracker = QuestTracker::default();
        let effects = vec![
            DialogueEffect::StartQuest("q1".into()),
            DialogueEffect::GiveItem("key".into(), 1),
            DialogueEffect::SetFlag("door_open".into()),
        ];
        let pending = apply_effects(&effects, &mut tracker);
        assert!(tracker.active_quests.contains(&"q1".into()));
        assert!(tracker.flags.contains(&"door_open".into()));
        assert_eq!(pending.len(), 1);
        match &pending[0] {
            PendingEffect::GiveItem(id, _) => assert_eq!(id, "key"),
            _ => panic!("预期 GiveItem"),
        }
    }

    #[test]
    fn condition_has_item_checks_inventory() {
        use crate::inventory::Inventory;
        let mut inventory = Inventory::default();
        inventory.give("torch", 1);
        let cond = DialogueCondition::HasItem("torch".into());
        assert!(cond.check(&QuestTracker::default(), &inventory));
        let cond2 = DialogueCondition::HasItem("axe".into());
        assert!(!cond2.check(&QuestTracker::default(), &inventory));
    }

    #[test]
    fn condition_no_item() {
        use crate::inventory::Inventory;
        let inventory = Inventory::default();
        let cond = DialogueCondition::NoItem("torch".into());
        assert!(cond.check(&QuestTracker::default(), &inventory));
    }

    #[test]
    fn condition_quest_complete() {
        let mut tracker = QuestTracker::default();
        let cond = DialogueCondition::QuestComplete("done_quest".into());
        assert!(!cond.check(&tracker, &Inventory::default()));
        tracker.completed_quests.push("done_quest".into());
        assert!(cond.check(&tracker, &Inventory::default()));
    }

    #[test]
    fn condition_quest_active() {
        let mut tracker = QuestTracker::default();
        let cond = DialogueCondition::QuestActive("active_q".into());
        assert!(!cond.check(&tracker, &Inventory::default()));
        tracker.active_quests.push("active_q".into());
        assert!(cond.check(&tracker, &Inventory::default()));
    }
}
