//! `NarratorMac`: AppKit's `NSSpeechSynthesizer`.

pub(super) struct Synthesizer;

impl Synthesizer {
    pub(super) fn open() -> Result<Synthesizer, String> {
        Err("NSSpeechSynthesizer is not ported yet".into())
    }
}

impl super::Speech for Synthesizer {
    fn say(&mut self, _: &str, _: bool, _: f32) {}
    fn clear(&mut self) {}
}
