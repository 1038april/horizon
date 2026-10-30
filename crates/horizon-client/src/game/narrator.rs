//! The narrator (`Minecraft.narrator`, horizon-narrator's `GameNarrator`):
//! opened with the client, closed with it, and kept in step with the options
//! it reads (`Options.narrator`, `getFinalSoundSourceVolume(VOICE)`).
use horizon_client::{settings::Settings, text::Text};
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
/// whatever the status (Off included).
pub(super) fn update_status(status: Status) {
    let announcement = Text::translate("options.narrator", &[])
        .append(Text::literal(" : "))
        .append(Text::translate(status.key(), &[]))
        .plain();
    horizon_narrator::update_status(status, &announcement);
}

/// `Minecraft.destroy`: `narrator.destroy()`.
pub(super) fn stop() {
    horizon_narrator::destroy();
}
