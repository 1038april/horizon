//! The Shader Options page of a `shaders/` pack: its options on the screens the pack lays out,
//! with the names its `lang` files give them, and the values the player picked.

use std::collections::BTreeMap;

use horizon_shaders::{Lang, PackOption, ScreenItem, Screens};

use crate::lang;
use crate::text::Text;

#[derive(Clone, Debug)]
pub struct ShaderOptions {
    /// The pack's file name in `shaderpacks/`.
    pub pack: String,
    pub options: BTreeMap<String, PackOption>,
    pub screens: Screens,
    pub lang: Lang,
    /// The values that differ from the defaults.
    pub chosen: BTreeMap<String, String>,
    /// The screens opened from the main one, innermost last.
    pub path: Vec<String>,
    /// Something changed since the page opened.
    pub changed: bool,
}

impl ShaderOptions {
    pub fn new(
        pack: String,
        options: BTreeMap<String, PackOption>,
        screens: Screens,
        lang: Lang,
        chosen: BTreeMap<String, String>,
    ) -> Self {
        let mut out = Self {
            pack,
            options,
            screens,
            lang,
            chosen: BTreeMap::new(),
            path: Vec::new(),
            changed: false,
        };
        for (name, value) in chosen {
            out.set(&name, value);
        }
        out.changed = false;
        out
    }

    /// The screen shown: the innermost opened, else the main one.
    pub fn screen(&self) -> &str {
        self.path.last().map_or("", String::as_str)
    }

    pub fn items(&self) -> &[ScreenItem] {
        self.screens
            .screen(self.screen())
            .map_or(&[], |(items, _)| items)
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.chosen
            .get(name)
            .or_else(|| self.options.get(name).map(|o| &o.default))
            .map(String::as_str)
    }

    fn set(&mut self, name: &str, value: String) {
        let Some(option) = self.options.get(name) else {
            return;
        };
        if value == option.default {
            self.chosen.remove(name);
        } else {
            self.chosen.insert(name.to_owned(), value);
        }
        self.changed = true;
    }

    /// The option's next value (the previous one with `back`), wrapping around.
    pub fn cycle(&mut self, name: &str, back: bool) {
        let (Some(option), Some(value)) = (self.options.get(name), self.value(name)) else {
            return;
        };
        let n = option.values.len();
        let at = option.values.iter().position(|v| v == value);
        let next = match (at, back) {
            (Some(i), false) => (i + 1) % n,
            (Some(i), true) => (i + n - 1) % n,
            (None, _) => 0,
        };
        let value = option.values[next].clone();
        self.set(name, value);
    }

    /// The profile after the current one (the first when none matches) with its values.
    pub fn next_profile(&mut self) {
        let profiles = &self.screens.profiles;
        if profiles.is_empty() {
            return;
        }
        let current = self.current_profile().map(str::to_owned);
        let at = profiles
            .iter()
            .position(|(n, _)| Some(n) == current.as_ref());
        let (_, values) = profiles[at.map_or(0, |i| (i + 1) % profiles.len())].clone();
        for (name, value) in values {
            self.set(&name, value);
        }
    }

    pub fn current_profile(&self) -> Option<&str> {
        self.screens
            .current_profile(|n| self.value(n).map(str::to_owned))
    }

    pub fn reset(&mut self) {
        if !self.chosen.is_empty() {
            self.chosen.clear();
            self.changed = true;
        }
    }

    fn named(&self, key: &str, or: &str) -> Text {
        Text::legacy(self.lang.get(key).unwrap_or(or))
    }

    /// The screen's title: its name in the pack's language, or its key.
    pub fn title(&self) -> Text {
        match self.screen() {
            "" => Text::literal(self.pack.clone()),
            screen => self.named(&format!("screen.{screen}"), screen),
        }
    }

    pub fn screen_label(&self, screen: &str) -> Text {
        self.named(&format!("screen.{screen}"), screen)
            .append("...")
    }

    pub fn profile_label(&self) -> Text {
        let value = match self.current_profile() {
            Some(name) => self.named(&format!("profile.{name}"), name),
            None => Text::literal(lang::get("generator.custom")),
        };
        Text::literal(format!("{}: ", lang::get("horizon.shader_options.profile"))).append(value)
    }

    /// `Name: value`, in the pack's words, the value yellow when it is not the default.
    pub fn option_label(&self, name: &str) -> Text {
        let Some(option) = self.options.get(name) else {
            return Text::literal(name.to_owned());
        };
        let value = self.value(name).unwrap_or_default();
        let shown = if option.switch {
            Text::literal(lang::get(if value == "true" {
                "options.on"
            } else {
                "options.off"
            }))
        } else {
            let text = self
                .lang
                .get(&format!("value.{name}.{value}"))
                .unwrap_or(value);
            let prefix = self.lang.get(&format!("prefix.{name}")).unwrap_or("");
            let suffix = self.lang.get(&format!("suffix.{name}")).unwrap_or("");
            Text::legacy(&format!("{prefix}{text}{suffix}"))
        };
        let shown = if self.chosen.contains_key(name) {
            shown.with_color(crate::text::YELLOW)
        } else {
            shown
        };
        self.named(&format!("option.{name}"), name)
            .append(": ")
            .append(shown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_shaders::{Macros, Properties};

    fn options() -> ShaderOptions {
        let switch = |name: &str, default: &str| PackOption {
            name: name.into(),
            default: default.into(),
            values: vec!["false".into(), "true".into()],
            switch: true,
            tested: true,
        };
        let options = BTreeMap::from([
            ("AO".to_owned(), switch("AO", "true")),
            (
                "LEVEL".to_owned(),
                PackOption {
                    name: "LEVEL".into(),
                    default: "2".into(),
                    values: vec!["1".into(), "2".into(), "3".into()],
                    switch: false,
                    tested: true,
                },
            ),
        ]);
        let text = "screen = <profile> [more]\nscreen.more = AO LEVEL\n\
                    profile.low = !AO LEVEL=1\nprofile.high = AO LEVEL=3\n";
        let screens = Screens::parse(&Properties::parse(text, &Macros::standard()), &options);
        ShaderOptions::new(
            "P.zip".into(),
            options,
            screens,
            Lang::default(),
            BTreeMap::from([("LEVEL".to_owned(), "2".to_owned())]),
        )
    }

    #[test]
    fn values_cycle_and_defaults_are_not_kept() {
        let mut o = options();
        assert!(o.chosen.is_empty() && !o.changed);
        o.cycle("LEVEL", false);
        o.cycle("LEVEL", false);
        assert_eq!(o.value("LEVEL"), Some("1"));
        o.cycle("LEVEL", true);
        o.cycle("AO", false);
        assert_eq!(
            o.chosen,
            BTreeMap::from([
                ("AO".to_owned(), "false".to_owned()),
                ("LEVEL".to_owned(), "3".to_owned())
            ])
        );
        assert!(o.changed);
        o.reset();
        assert!(o.chosen.is_empty());
    }

    #[test]
    fn profiles_follow_the_values() {
        let mut o = options();
        assert_eq!(o.current_profile(), None);
        o.next_profile();
        assert_eq!(o.current_profile(), Some("low"));
        o.next_profile();
        assert_eq!(o.current_profile(), Some("high"));
        assert_eq!(o.value("LEVEL"), Some("3"));
        o.next_profile();
        assert_eq!(o.current_profile(), Some("low"));
    }
}
