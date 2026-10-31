//! `NarratorLinux`: flite's `cmu_us_kal16` voice.

pub(super) struct Flite;

impl Flite {
    pub(super) fn open() -> Result<Flite, String> {
        Err("flite is not ported yet".into())
    }
}

impl super::Speech for Flite {
    fn say(&mut self, _: &str, _: bool, _: f32) {}
    fn clear(&mut self) {}
}
