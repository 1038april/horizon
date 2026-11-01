//! A client mod's view and its own items: the WebAssembly mod API's `world.raycast` (across the distant
//! terrain), `input.held`, `input.claim-items` and `input.poll` from the Stink-O/horizon fork, as native helpers.
use super::Game;
use std::collections::BTreeMap;

/// `input.held-item`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeHeldItem {
    pub(crate) id: String,
    pub(crate) count: u32,
    pub(crate) slot: u32,
    pub(crate) custom_name: Option<String>,
    pub(crate) custom_data: Option<String>,
}
/// `world.hit`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeHit {
    pub(crate) block_x: i32,
    pub(crate) block_y: i32,
    pub(crate) block_z: i32,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) block: String,
}
/// `input.event`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum NativeModEvent {
    UseStart(NativeHeldItem, u32),
    UseStop(NativeHeldItem, u32),
    ScreenClick(f32, f32, u32),
    ScreenClosed,
}
/// `world.raycast`'s reach.
pub(crate) const RAYCAST_MAX: f32 = 1024.0;
/// `input.claim-items`: the ids a mod may claim.
const CLAIMS_MAX: usize = 64;
/// Events kept for a mod between two polls.
const EVENTS_MAX: usize = 256;

/// The input state the fork's runtime kept per mod.
#[derive(Default)]
pub(super) struct ModInput {
    claims: BTreeMap<String, Vec<String>>,
    events: BTreeMap<String, Vec<NativeModEvent>>,
    /// The mod whose item is in use.
    using: Option<String>,
}

impl ModInput {
    fn claim(&mut self, mod_id: &str, ids: &[String]) {
        let ids = ids.iter().take(CLAIMS_MAX).cloned().collect();
        self.claims.insert(mod_id.to_owned(), ids);
    }
    /// The mod that claims `item`, the first in id order.
    fn claimant(&self, item: &str) -> Option<&str> {
        self.claims
            .iter()
            .find(|(_, ids)| ids.iter().any(|i| i == item))
            .map(|(id, _)| id.as_str())
    }
    fn push(&mut self, mod_id: &str, event: NativeModEvent) {
        let events = self.events.entry(mod_id.to_owned()).or_default();
        if events.len() < EVENTS_MAX {
            events.push(event);
        }
    }
    fn poll(&mut self, mod_id: &str) -> Vec<NativeModEvent> {
        self.events.remove(mod_id).unwrap_or_default()
    }
}

#[allow(dead_code)]
impl Game<'_> {
    /// `input.held`: the stack in the main hand. API @API:native_claimed_items@.
    pub(super) fn native_held_item(&self) -> Option<NativeHeldItem> {
        self.player.as_ref()?;
        self.mod_held(self.selected)
    }
    /// The stack in inventory slot `slot` as mods see it: its name and custom data as SNBT.
    fn mod_held(&self, slot: usize) -> Option<NativeHeldItem> {
        let stack = self.stack_in(slot)?;
        let extra = stack.extra.as_deref();
        Some(NativeHeldItem {
            id: stack.item.to_string(),
            count: stack.count,
            slot: slot as u32,
            custom_name: extra
                .and_then(|e| e.custom_name.as_ref())
                .map(horizon_client::snbt::to_snbt),
            custom_data: extra
                .and_then(|e| e.other("minecraft:custom_data"))
                .map(horizon_client::snbt::to_snbt),
        })
    }
    /// `world.raycast`: the first block along the view within `distance` (at most 1024), through distant terrain. API @API:native_claimed_items@.
    pub(super) fn native_raycast(&self, distance: f32) -> Option<NativeHit> {
        let (d, hit) = self.native_view_hit()?;
        (d <= distance.clamp(0.0, RAYCAST_MAX)).then_some(hit)
    }
    /// The first block along the view within `RAYCAST_MAX`, and its distance. API @API:native_claimed_items@.
    pub(super) fn native_view_hit(&self) -> Option<(f32, NativeHit)> {
        self.player.as_ref()?;
        let eye = self.eye_position()?;
        let dir = super::view_vector(self.yaw, self.pitch);
        let reach = f64::from(RAYCAST_MAX);
        let (pos, point, state) = match self.clip_ray(eye, dir, reach) {
            Some(hit) => (
                hit.pos,
                hit.point,
                self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]),
            ),
            None => self.lod_ray(eye, dir, reach)?,
        };
        let distance = (0..3)
            .map(|i| (point[i] - eye[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        Some((
            distance as f32,
            NativeHit {
                block_x: pos[0],
                block_y: pos[1],
                block_z: pos[2],
                x: point[0],
                y: point[1],
                z: point[2],
                block: self.res.facts().get(state).name.to_string(),
            },
        ))
    }
    /// A ray over the distant terrain (`Lod::ground`) past the loaded chunks, a block at a time.
    fn lod_ray(
        &self,
        eye: [f64; 3],
        dir: [f64; 3],
        reach: f64,
    ) -> Option<([i32; 3], [f64; 3], horizon_world::registry::BlockStateId)> {
        let lod = self.lod.as_ref()?;
        let mut t = 0.0;
        while t <= reach {
            let p = [
                eye[0] + dir[0] * t,
                eye[1] + dir[1] * t,
                eye[2] + dir[2] * t,
            ];
            let (x, z) = (p[0].floor() as i32, p[2].floor() as i32);
            if !self.world.loaded(x, z)
                && let Some((height, state)) = lod.ground(x, z)
                && p[1] <= f64::from(height)
            {
                return Some(([x, height - 1, z], [p[0], f64::from(height), p[2]], state));
            }
            t += 1.0;
        }
        None
    }
    /// `input.claim-items`: right-clicks with these items (at most 64) are mod `mod_id`'s. API @API:native_claimed_items@.
    pub(super) fn native_claim_items(&mut self, mod_id: &str, ids: &[String]) {
        self.native_input.claim(mod_id, ids);
    }
    /// `input.poll`: mod `mod_id`'s events since the last poll (at most 256). API @API:native_claimed_items@.
    pub(super) fn native_poll(&mut self, mod_id: &str) -> Vec<NativeModEvent> {
        self.native_input.poll(mod_id)
    }
    pub(super) fn push_mod_event(&mut self, mod_id: &str, event: NativeModEvent) {
        self.native_input.push(mod_id, event);
    }
    /// `use_hand`: a right-click with an item a mod claims starts its use for
    /// that mod and nothing else happens. True when a mod took it.
    pub(super) fn mod_use_start(&mut self) -> bool {
        if self.guest_use.is_some() || self.using.is_some() {
            return false;
        }
        let slot = self.held_slot();
        let Some(item) = self.mod_held(slot) else {
            return false;
        };
        let Some(id) = self.native_input.claimant(&item.id).map(str::to_owned) else {
            return false;
        };
        self.using = Some((slot, 0));
        self.push_mod_event(&id, NativeModEvent::UseStart(item, 0));
        self.native_input.using = Some(id);
        true
    }
    /// `tick_used_item` for a claimed item: counts the ticks held and ends
    /// the use when the button comes up. True when the item in use is one.
    pub(super) fn mod_use_tick(&mut self, slot: usize, ticks: u32, released: bool) -> bool {
        let Some(id) = self.native_input.using.clone() else {
            return false;
        };
        let still_claimed = self
            .stack_in(slot)
            .is_some_and(|s| self.native_input.claimant(s.item.as_ref()) == Some(id.as_str()));
        if released || !still_claimed {
            if let Some(item) = self.mod_held(slot) {
                self.push_mod_event(&id, NativeModEvent::UseStop(item, ticks));
            }
            self.native_input.using = None;
            self.using = None;
        } else {
            self.using = Some((slot, ticks + 1));
        }
        true
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn items_are_claimed_by_the_first_mod_in_id_order() {
        let mut input = ModInput::default();
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        input.claim("zeta", &ids(&["orbital:remote", "minecraft:stick"]));
        input.claim("alpha", &ids(&["minecraft:stick"]));
        assert_eq!(input.claimant("minecraft:stick"), Some("alpha"));
        assert_eq!(input.claimant("orbital:remote"), Some("zeta"));
        assert_eq!(input.claimant("minecraft:dirt"), None);
        input.claim("alpha", &[]);
        assert_eq!(input.claimant("minecraft:stick"), Some("zeta"));
        let many: Vec<String> = (0..100).map(|i| format!("m:item{i}")).collect();
        input.claim("alpha", &many);
        assert_eq!(input.claimant("m:item63"), Some("alpha"));
        assert_eq!(input.claimant("m:item64"), None, "64 at most");
    }

    #[test]
    fn events_wait_for_their_mod_up_to_a_limit() {
        let mut input = ModInput::default();
        for _ in 0..EVENTS_MAX + 10 {
            input.push("a", NativeModEvent::ScreenClosed);
        }
        input.push("b", NativeModEvent::ScreenClosed);
        assert_eq!(input.poll("a").len(), EVENTS_MAX);
        assert!(input.poll("a").is_empty());
        assert_eq!(input.poll("b"), vec![NativeModEvent::ScreenClosed]);
    }
}
