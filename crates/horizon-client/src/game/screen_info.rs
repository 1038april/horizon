//! What the in-game screens narrate where vanilla's override
//! `getNarrationMessage` or `updateNarrationState`: the `ScreenInfo`s their
//! drawers begin. Built only while the narrator records (`begin_screen`'s
//! closure), with the translator given so tests can see the keys.
use horizon_client::widgets::{ElementType, ScreenInfo, join_for_narration};

/// A translation of `key` with plain arguments (`I18n.get`).
pub(super) type Tr<'a> = &'a dyn Fn(&str, &[&str]) -> String;

/// The game's translator.
pub(super) fn tr(key: &str, args: &[&str]) -> String {
    horizon_client::screen_narrator::translate(key, args)
}

/// `ChatScreen.updateNarrationState` (and `InBedChatScreen`'s): its title,
/// how to send (and that the chat is restricted), and the message typed so
/// far one level in. No widget and no navigation.
pub(super) fn chat(in_bed: bool, value: &str, restricted: bool, tr: Tr<'_>) -> ScreenInfo {
    let usage = tr("chat_screen.usage", &[]);
    let usage = if restricted {
        join_for_narration(&[&usage, &tr("chat_screen.restricted.narration", &[])])
    } else {
        usage
    };
    let id = if in_bed { "chat/in_bed" } else { "chat" };
    let info = ScreenInfo::new(id, tr("chat_screen.title", &[]))
        .without_widgets()
        .without_navigation()
        .element(ElementType::Usage, 0, usage);
    if value.is_empty() {
        info
    } else {
        info.element(ElementType::Title, 1, tr("chat_screen.message", &[value]))
    }
}

/// `LevelLoadingScreen`: no title and no navigation; `updateNarratedWidget`
/// says the server's progress (`loading.progress`, floored to a percent) in
/// the title's place when there is one, and `extractRenderState` narrates
/// what changed every 2 s.
pub(super) fn level_loading(progress: Option<f32>, tr: Tr<'_>) -> ScreenInfo {
    let info = ScreenInfo::new("level_loading", "")
        .without_widgets()
        .without_navigation()
        .repeat_every(2000);
    match progress {
        Some(p) => {
            let percent = ((p * 100.0).floor() as i32).to_string();
            info.element(ElementType::Title, 0, tr("loading.progress", &[&percent]))
        }
        None => info,
    }
}

/// `ConfirmScreen.getNarrationMessage` (`PackConfirmScreen`,
/// `ConfirmLinkScreen`, `DialogScreen.WarningScreen`): the title and the
/// message.
pub(super) fn confirm(id: &str, title: &str, message: &str) -> ScreenInfo {
    ScreenInfo::new(id, join_for_narration(&[title, message]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "key(arg, …)": which key and arguments the narration says.
    fn keys(key: &str, args: &[&str]) -> String {
        if args.is_empty() {
            key.to_owned()
        } else {
            format!("{key}({})", args.join(", "))
        }
    }

    #[test]
    fn the_chat_narrates_its_usage_and_the_message_typed_but_no_widget() {
        let empty = chat(false, "", false, &keys);
        assert_eq!(empty.id, "chat");
        assert_eq!(empty.title, "chat_screen.title");
        assert!(!empty.widgets && !empty.navigation);
        assert_eq!(
            empty.extra,
            [(ElementType::Usage, 0, vec!["chat_screen.usage".to_owned()])]
        );
        let typed = chat(true, "hello", true, &keys);
        assert_eq!(typed.id, "chat/in_bed");
        assert_eq!(
            typed.extra,
            [
                (
                    ElementType::Usage,
                    0,
                    vec!["chat_screen.usage. chat_screen.restricted.narration".to_owned()]
                ),
                (
                    ElementType::Title,
                    1,
                    vec!["chat_screen.message(hello)".to_owned()]
                ),
            ]
        );
    }

    #[test]
    fn level_loading_says_its_progress_in_the_title_place_every_two_seconds() {
        let waiting = level_loading(None, &keys);
        assert_eq!(waiting.title, "");
        assert!(waiting.extra.is_empty());
        assert!(!waiting.navigation);
        assert_eq!(waiting.repeat_ms, Some(2000));
        let loading = level_loading(Some(0.567), &keys);
        assert_eq!(
            loading.extra,
            [(
                ElementType::Title,
                0,
                vec!["loading.progress(56)".to_owned()]
            )]
        );
    }

    #[test]
    fn confirm_screens_join_their_title_and_message() {
        assert_eq!(
            confirm("link", "Open this?", "https://example.com").title,
            "Open this?. https://example.com"
        );
    }
}
