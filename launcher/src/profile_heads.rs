//! Profile-head I/O stays on workers; every account surface shares the same pixel-sharp cache.
use super::*;
use gpui::{img, RenderImage};
use std::{cell::RefCell, path::PathBuf, sync::Arc};

#[derive(Default)]
pub(super) struct Heads(HashMap<String, Head>);

struct Head {
    url: Option<String>,
    pending: bool,
    face: Option<Vec<[u8; 4]>>,
    images: RefCell<HashMap<(u32, u32), Arc<RenderImage>>>,
}

impl Heads {
    pub fn request(&mut self, accounts: &[account::Account], resources: PathBuf, tx: &Sender<Msg>) {
        self.0
            .retain(|uuid, _| accounts.iter().any(|a| &a.uuid == uuid));
        for account in accounts {
            let url = account.skin.as_ref().map(|s| s.0.clone());
            if self
                .0
                .get(&account.uuid)
                .is_some_and(|h| h.url == url && (h.pending || h.face.is_some()))
            {
                continue;
            }
            let previous = self.0.remove(&account.uuid).and_then(|h| h.face);
            self.0.insert(
                account.uuid.clone(),
                Head {
                    url: url.clone(),
                    pending: true,
                    face: previous,
                    images: RefCell::default(),
                },
            );
            let (account, resources, tx) = (account.clone(), resources.clone(), tx.clone());
            std::thread::spawn(move || {
                let face = horizon_launcher::heads::profile_face(
                    &account,
                    &backend::data_dir().join("profile-heads"),
                    &resources,
                );
                let _ = tx.send(Msg::AccountHead(account.uuid, url, face));
            });
        }
    }

    pub fn loaded(&mut self, uuid: &str, url: &Option<String>, face: Option<Vec<[u8; 4]>>) {
        if let Some(head) = self.0.get_mut(uuid).filter(|h| &h.url == url) {
            head.pending = false;
            if face.is_some() {
                head.face = face;
            }
            head.images.get_mut().clear();
        }
    }

    pub fn draw(&self, uuid: Option<&str>, size: f32, scale: f32, t: &Theme) -> Div {
        let image = uuid.and_then(|uuid| self.0.get(uuid)).and_then(|head| {
            let face = head.face.as_ref()?;
            Some(
                head.images
                    .borrow_mut()
                    .entry((size.to_bits(), scale.to_bits()))
                    .or_insert_with(|| art::bake(face, 8, 8, size / 8., scale))
                    .clone(),
            )
        });
        let d = div()
            .flex_none()
            .size(px(size))
            .overflow_hidden()
            .bg(t.well)
            .flex()
            .items_center()
            .justify_center();
        match image {
            Some(image) => d.child(img(image).size(px(size))),
            None => d.child(blocks::icon("user", size * 0.6, t.muted)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_skin_results_are_ignored_and_scaled_images_are_reused() {
        let mut heads = Heads::default();
        let current = Some("current-skin".to_string());
        heads.0.insert(
            "player".into(),
            Head {
                url: current.clone(),
                pending: true,
                face: None,
                images: RefCell::default(),
            },
        );
        heads.loaded(
            "player",
            &Some("old-skin".into()),
            Some(vec![[255, 0, 0, 255]; 64]),
        );
        assert!(heads.0["player"].face.is_none());
        heads.loaded("player", &current, Some(vec![[0, 255, 0, 255]; 64]));
        let theme = Theme::for_appearance(
            backend::Appearance::Dark,
            horizon_launcher::themes::Accent::Violet,
            gpui::WindowAppearance::Dark,
        );
        let _ = heads.draw(Some("player"), 32., 2., &theme);
        let key = (32f32.to_bits(), 2f32.to_bits());
        let image = heads.0["player"].images.borrow()[&key].clone();
        assert_eq!(image.size(0).width.0, 64);
        assert_eq!(image.size(0).height.0, 64);
        let _ = heads.draw(Some("player"), 32., 2., &theme);
        assert!(Arc::ptr_eq(
            &image,
            &heads.0["player"].images.borrow()[&key]
        ));
    }
}
