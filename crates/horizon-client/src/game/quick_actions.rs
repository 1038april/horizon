//! The Quick Actions screen (`key.quickActions`, default G): a non-pausing
//! menu opened over the world. Datapacks define the buttons via
//! `data/<namespace>/dialog/*.json` files with `"type": "minecraft:multi_action"`.
//! Clicking a button runs its action as a command.
use super::{Game, Screen};
use horizon_client::{
    datapack::{self, DataPack},
    hud::HudBatch,
    keybinds,
    text::Text,
};
use serde::Deserialize;
use std::collections::BTreeSet;

/// Panel size.
const W: f32 = 256.0;
const H: f32 = 180.0;
const BTN: f32 = 52.0;
const GAP: f32 = 16.0;

/// One action button defined by a datapack dialog.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct DialogAction {
    label: String,
    tooltip: Option<String>,
    action_id: String,
}

/// A multi_action dialog from a datapack (`data/<ns>/dialog/<name>.json`).
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct DialogFile {
    #[allow(dead_code)]
    r#type: String,
    title: serde_json::Value,
    #[allow(dead_code)]
    body: serde_json::Value,
    #[allow(dead_code)]
    columns: Option<usize>,
    actions: Vec<DialogActionEntry>,
}

#[derive(Deserialize, Debug)]
struct DialogActionEntry {
    label: serde_json::Value,
    #[allow(dead_code)]
    tooltip: Option<serde_json::Value>,
    action: DialogActionInner,
}

#[derive(Deserialize, Debug)]
struct DialogActionInner {
    #[allow(dead_code)]
    r#type: String,
    id: String,
}

impl Game<'_> {
    /// Load all enabled datapack dialog actions for the Quick Actions menu.
    fn load_dialog_actions(&self) -> Vec<DialogAction> {
        let mut actions: Vec<DialogAction> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();

        let packs = datapack::discover(&self.world_root);
        let enabled = datapack::enabled_ids(&self.world_root, &packs);

        // Built-in vanilla pack first, then enabled world packs in order.
        let ordered: Vec<&DataPack> = packs
            .iter()
            .filter(|p| p.id == "vanilla" || enabled.contains(&p.id))
            .collect();

        for pack in ordered {
            let files = pack.list("data/");
            // Collect dialog json files under data/<namespace>/dialog/.
            let mut dialog_files: Vec<String> = files
                .iter()
                .filter(|f| f.starts_with("data/") && f.contains("/dialog/") && f.ends_with(".json"))
                .cloned()
                .collect();
            dialog_files.sort();

            for file in dialog_files {
                if !seen.insert(file.clone()) {
                    continue;
                }
                let bytes = match pack.read(&file) {
                    Some(b) => b,
                    None => continue,
                };
                let dialog = match serde_json::from_slice::<DialogFile>(&bytes) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                if dialog.r#type != "minecraft:multi_action" {
                    continue;
                }
                for entry in &dialog.actions {
                    let action_id = entry.action.id.clone();
                    if action_id.is_empty() {
                        continue;
                    }
                    let label = text_of(&entry.label);
                    let tooltip = entry.tooltip.as_ref().map(text_of);
                    actions.push(DialogAction {
                        label,
                        tooltip,
                        action_id,
                    });
                }
            }
        }
        actions
    }

    pub(super) fn open_quick_actions(&mut self) {
        self.screen = Screen::QuickActions;
        self.keys.clear();
        self.left = false;
        self.right = false;
        self.mining = None;
        self.capture(false);
    }

    pub(super) fn close_quick_actions(&mut self) {
        self.screen = Screen::Playing;
        self.capture(true);
    }

    pub(super) fn draw_quick_actions(&mut self, batch: &mut HudBatch) {
        let (w, h) = (batch.width as f32, batch.height as f32);
        let x = (w - W) / 2.0;
        let y = (h - H) / 2.0;
        let atlas = &self.gui.atlas;

        // Dimmed background behind the panel.
        batch.sprite_region(
            atlas,
            "gui/sprites/quick_actions/backdrop",
            [0.0, 0.0, w, h],
            w,
            h,
        );

        // Panel.
        batch.sprite_region(
            atlas,
            "gui/sprites/quick_actions/panel",
            [x, y, W, H],
            W,
            H,
        );

        // Title from the first datapack dialog, or a default.
        let actions = self.load_dialog_actions();
        let title_text = if actions.is_empty() {
            Text::translate("quickActions.title", &[])
        } else {
            Text::literal("Quick Actions")
        };
        let tw = HudBatch::text_width(atlas, &title_text.plain());
        batch.text_styled(
            atlas,
            &title_text,
            (w - tw) / 2.0,
            y + 20.0,
            u32::MAX,
            true,
        );

        if actions.is_empty() {
            let hint = Text::translate("quickActions.none", &[]);
            let hw = HudBatch::text_width(atlas, &hint.plain());
            batch.text_styled(
                atlas,
                &hint,
                (w - hw) / 2.0,
                y + H / 2.0,
                u32::MAX,
                true,
            );
            return;
        }

        // Layout buttons in a grid.
        let cols = 2usize;
        let rows = ((actions.len() + cols - 1) / cols).max(1);
        let total_w = cols as f32 * BTN + (cols as f32 - 1.0) * GAP;
        let total_h = rows as f32 * BTN + (rows as f32 - 1.0) * GAP;
        let bx = x + (W - total_w) / 2.0;
        let by = y + (H - total_h) / 2.0 + 20.0;

        for (i, action) in actions.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let bx2 = bx + col as f32 * (BTN + GAP);
            let by2 = by + row as f32 * (BTN + GAP);
            batch.sprite_region(
                atlas,
                "gui/sprites/quick_actions/button",
                [bx2, by2, BTN, BTN],
                BTN,
                BTN,
            );
            let label = Text::literal(&action.label);
            let lw = HudBatch::text_width(atlas, &label.plain());
            batch.text_styled(
                atlas,
                &label,
                bx2 + (BTN - lw) / 2.0,
                by2 + BTN + 4.0,
                u32::MAX,
                true,
            );
        }

        // Hint: press G or Escape to close.
        let hint = Text::translate(
            "quickActions.close",
            &[keybinds::display_name(
                self.settings.keys.get("key.quickActions"),
            )],
        );
        let hw = HudBatch::text_width(atlas, &hint.plain());
        batch.text_styled(
            atlas,
            &hint,
            (w - hw) / 2.0,
            y + H - 16.0,
            u32::MAX,
            true,
        );
    }

    /// Called on a left mouse click while the Quick Actions screen is open.
    /// Returns `true` if a button was clicked (the click is consumed).
    pub(super) fn quick_actions_click(&mut self) -> bool {
        let (w, h) = (
            self.renderer.as_ref().map_or(0, |r| r.config.width) as f32,
            self.renderer.as_ref().map_or(0, |r| r.config.height) as f32,
        );
        let x = (w - W) / 2.0;
        let y = (h - H) / 2.0;
        let actions = self.load_dialog_actions();
        if actions.is_empty() {
            return false;
        }
        let cols = 2usize;
        let rows = ((actions.len() + cols - 1) / cols).max(1);
        let total_w = cols as f32 * BTN + (cols as f32 - 1.0) * GAP;
        let total_h = rows as f32 * BTN + (rows as f32 - 1.0) * GAP;
        let bx = x + (W - total_w) / 2.0;
        let by = y + (H - total_h) / 2.0 + 20.0;

        let cx = self.cursor.0;
        let cy = self.cursor.1;

        for (i, action) in actions.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let bx2 = bx + col as f32 * (BTN + GAP);
            let by2 = by + row as f32 * (BTN + GAP);
            if cx >= bx2 && cx < bx2 + BTN && cy >= by2 && cy < by2 + BTN {
                self.quick_action_pressed(&action.action_id);
                return true;
            }
        }
        false
    }

    fn quick_action_pressed(&mut self, action_id: &str) {
        // Run the action as a command (vanilla: custom dialog actions run commands).
        let command = format!("/{}", action_id);
        self.run_command(&command);
        self.close_quick_actions();
    }
}

/// Extract plain text from a dialog text component.
fn text_of(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_owned();
    }
    if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
        return t.to_owned();
    }
    String::new()
}
