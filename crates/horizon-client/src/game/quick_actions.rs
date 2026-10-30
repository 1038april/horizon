//! The Quick Actions screen (`key.quickActions`, default G): a non-pausing
//! menu opened over the world that offers quick player actions, like
//! vanilla's Quick Actions dialog.
use super::{Game, Screen};
use horizon_client::{hud::HudBatch, keybinds, text::Text};

/// One quick action button: id, translation key and sprite.
struct Action {
    id: &'static str,
    title: &'static str,
    sprite: &'static str,
}

const ACTIONS: [Action; 4] = [
    Action {
        id: "emote",
        title: "quickActions.emote",
        sprite: "gui/sprites/quick_actions/emote",
    },
    Action {
        id: "quick_chat",
        title: "quickActions.quick_chat",
        sprite: "gui/sprites/quick_actions/chat",
    },
    Action {
        id: "camera",
        title: "quickActions.camera",
        sprite: "gui/sprites/quick_actions/camera",
    },
    Action {
        id: "gesture",
        title: "quickActions.gesture",
        sprite: "gui/sprites/quick_actions/gesture",
    },
];

/// Panel size.
const W: f32 = 256.0;
const H: f32 = 180.0;
const BTN: f32 = 52.0;
const GAP: f32 = 16.0;

impl Game<'_> {
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

        // Title.
        let title = Text::translate("quickActions.title", &[]);
        let tw = HudBatch::text_width(atlas, &title.plain());
        batch.text_styled(
            atlas,
            &title,
            (w - tw) / 2.0,
            y + 20.0,
            u32::MAX,
            true,
        );

        // Buttons: two rows of two.
        let cols = 2;
        let rows = 2;
        let total_w = cols as f32 * BTN + (cols as f32 - 1.0) * GAP;
        let total_h = rows as f32 * BTN + (rows as f32 - 1.0) * GAP;
        let bx = x + (W - total_w) / 2.0;
        let by = y + (H - total_h) / 2.0 + 20.0;

        for (row, col) in (0..rows).flat_map(|r| (0..cols).map(move |c| (r, c))) {
            let i = row * cols + col;
            if i >= ACTIONS.len() {
                break;
            }
            let action = &ACTIONS[i];
            let bx2 = bx + col as f32 * (BTN + GAP);
            let by2 = by + row as f32 * (BTN + GAP);
            batch.sprite_region(
                atlas,
                action.sprite,
                [bx2, by2, BTN, BTN],
                BTN,
                BTN,
            );
            let label = Text::translate(action.title, &[]);
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
        let cols = 2;
        let rows = 2;
        let total_w = cols as f32 * BTN + (cols as f32 - 1.0) * GAP;
        let total_h = rows as f32 * BTN + (rows as f32 - 1.0) * GAP;
        let bx = x + (W - total_w) / 2.0;
        let by = y + (H - total_h) / 2.0 + 20.0;

        let cx = self.cursor.0;
        let cy = self.cursor.1;

        for (row, col) in (0..rows).flat_map(|r| (0..cols).map(move |c| (r, c))) {
            let i = row * cols + col;
            if i >= ACTIONS.len() {
                break;
            }
            let bx2 = bx + col as f32 * (BTN + GAP);
            let by2 = by + row as f32 * (BTN + GAP);
            if cx >= bx2 && cx < bx2 + BTN && cy >= by2 && cy < by2 + BTN {
                self.quick_action_pressed(ACTIONS[i].id);
                return true;
            }
        }
        false
    }

    fn quick_action_pressed(&mut self, id: &str) {
        match id {
            "emote" => {
                // Open the emote picker (vanilla: EmoteScreen).
                // Placeholder: no emote system yet, just close.
                log::debug!("quickActions: emote (not yet implemented)");
            }
            "quick_chat" => {
                // Open quick chat (vanilla: quick chat screen).
                log::debug!("quickActions: quick chat (not yet implemented)");
            }
            "camera" => {
                // Toggle camera mode (vanilla: F5 equivalent).
                self.cycle_camera();
            }
            "gesture" => {
                // Placeholder for a gesture/emote action.
                log::debug!("quickActions: gesture (not yet implemented)");
            }
            _ => {}
        }
        self.close_quick_actions();
    }
}
