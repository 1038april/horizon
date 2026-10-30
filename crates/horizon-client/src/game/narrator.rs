//! The narrator (`Minecraft.narrator`, horizon-narrator's `GameNarrator`):
//! opened with the client, closed with it, and kept in step with the options
//! it reads (`Options.narrator`, `getFinalSoundSourceVolume(VOICE)`); and its
//! `NARRATOR_TOGGLE` toast.
use super::Game;
use horizon_client::{
    hud::{HudAtlas, HudBatch},
    settings::Settings,
    text::Text,
    toasts::SystemToast,
};
use horizon_narrator::Status;

/// `Minecraft`'s constructor: `new GameNarrator`, then
/// `narrator.checkStatus(options.narrator().get() != OFF)`.
pub(super) fn start(settings: &Settings) {
    horizon_narrator::init();
    sync(settings);
    check_status(settings.narrator() != Status::Off);
}

/// `GameNarrator.checkStatus`: the narrator is wanted but its speech didn't
/// open. Vanilla asks in a message box whether to go on; Horizon has no
/// native message box, so it logs vanilla's message and goes on, as the
/// box's Continue does (the option then reads "Not Available").
fn check_status(required_active: bool) {
    if required_active && !horizon_narrator::is_active() {
        log::error!(
            "Failed to initialize text-to-speech library; continuing without the narrator. \
             If this problem persists, please report it at bugs.mojang.com"
        );
    }
}

/// The options the narrator reads, after they load or change: its status and
/// the voice volume (master times voice).
pub(super) fn sync(settings: &Settings) {
    horizon_narrator::set_status(settings.narrator());
    horizon_narrator::set_volume(settings.final_volume("voice"));
}

/// `GameNarrator.updateNarratorStatus`, once the narrator option holds
/// `status`: what was being said stops and "Narrator : <status>" is said,
/// whatever the status (Off included). Then the `NARRATOR_TOGGLE` toast goes
/// up, or the one still up takes the new text (`SystemToast.addOrUpdate`).
pub(super) fn update_status(status: Status, toast: &mut Option<SystemToast>) {
    let announcement = Text::translate("options.narrator", &[])
        .append(Text::literal(" : "))
        .append(Text::translate(status.key(), &[]))
        .plain();
    horizon_narrator::update_status(status, &announcement);
    let (title, message) = horizon_narrator::toast(status, horizon_narrator::is_active());
    let tr = |key: &str| Text::translate(key, &[]).plain();
    let (title, message) = (tr(title), message.map(tr));
    match toast {
        Some(toast) => toast.reset(title, message),
        None => *toast = Some(SystemToast::new(title, message)),
    }
}

/// `ToastManager.update` and `extractRenderState` for the narrator toast: it
/// takes the first slot `used` leaves free, slides, and is drawn there unless
/// `hidden` (F1). The visibility it switched to, for its sound.
pub(super) fn draw_toast(
    toast: &mut Option<SystemToast>,
    used: u32,
    multiplier: f64,
    atlas: &HudAtlas,
    batch: &mut HudBatch,
    hidden: bool,
) -> Option<bool> {
    let shown = toast.as_mut()?;
    if shown.slot.is_none() {
        shown.slot = Some((0..Game::TOAST_SLOTS).find(|s| used & (1 << s) == 0)?);
    }
    let (sound, finished) = shown.update(multiplier);
    if finished {
        *toast = None;
    } else if let (false, Some(slot)) = (hidden, shown.slot) {
        shown.draw(atlas, batch, 32.0 * slot as f32);
    }
    sound
}

impl Game<'_> {
    /// The narrator option changed, on the options screen or by its hotkey.
    pub(super) fn narrator_status_changed(&mut self) {
        update_status(self.settings.narrator(), &mut self.narrator_toast);
    }
    /// The narrator toast over whatever is on screen, the title's own toast
    /// keeping the first slot.
    pub(super) fn draw_narrator_toast(&mut self, batch: &mut HudBatch) {
        let title = self.title.as_ref().is_some_and(|t| t.has_toast());
        let used = self.toast_slots_used() | u32::from(title);
        let multiplier = f64::from(self.settings.notification_display_time);
        let sound = draw_toast(
            &mut self.narrator_toast,
            used,
            multiplier,
            &self.gui.atlas,
            batch,
            self.ui.hide_gui,
        );
        if let Some(show) = sound {
            let event = if show { "ui.toast.in" } else { "ui.toast.out" };
            self.audio
                .play(&format!("minecraft:{event}"), None, 1.0, 1.0);
        }
    }
}

/// `Minecraft.destroy`: `narrator.destroy()`.
pub(super) fn stop() {
    horizon_narrator::destroy();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SystemToast.addOrUpdate`: one narrator toast, reset where it is.
    #[test]
    fn a_second_change_updates_the_toast_still_up() {
        let mut toast = None;
        update_status(Status::All, &mut toast);
        toast.as_mut().expect("the first change shows a toast").slot = Some(2);
        update_status(Status::Off, &mut toast);
        assert_eq!(toast.expect("still the one toast").slot, Some(2));
    }
}
