//! The player's motion for mods: a push for the local player (`world.push-player`) and a player's flying speed
//! (`world.set-flying-speed`), from the Stink-O/horizon fork.
use super::Game;

/// `world.push-player`: each component of a push is clamped to this, blocks a tick.
const PUSH_MAX: f32 = 4.0;
/// `world.set-flying-speed`: the fastest a mod may set (vanilla's is 0.05).
const FLYING_MAX: f32 = 50.0;
/// Vanilla's creative flying speed (`Abilities.flyingSpeed`).
const FLYING_VANILLA: f32 = 0.05;

/// The velocity after a push: `kick` added, each component clamped; none when one of `kick`'s isn't a number.
fn pushed(velocity: [f64; 3], kick: [f32; 3]) -> Option<[f64; 3]> {
    if !kick.iter().all(|v| v.is_finite()) {
        return None;
    }
    Some(std::array::from_fn(|i| {
        velocity[i] + f64::from(kick[i].clamp(-PUSH_MAX, PUSH_MAX))
    }))
}

#[allow(dead_code)]
impl Game<'_> {
    /// `world.push-player`, for a client mod: added to the local player's velocity, in blocks a tick, as an
    /// explosion's knockback is (the window's player moves itself, as vanilla's client does). Each component is
    /// clamped to -4..4; a spectator, or a value that is not a number, is left alone. API @API:native_push_player@.
    pub(super) fn native_push_player(&mut self, kick: [f32; 3]) {
        if let Some(p) = self.player.as_mut()
            && !p.spectator
            && let Some([x, y, z]) = pushed([p.velocity.x, p.velocity.y, p.velocity.z], kick)
        {
            p.velocity = horizon_core::geometry::Vec3::new(x, y, z);
            if kick[1] > 0.0 {
                p.ground.on_ground = false;
            }
        }
    }
    /// `world.set-flying-speed`, for a server mod: `player`'s creative flying speed (vanilla's 0.05 when none, at
    /// most 50), kept on the server's player as vanilla keeps its abilities and sent to their client, so a respawn
    /// or a game mode change keeps it. False for a player not here or a client of a server. API @API:native_push_player@.
    pub(super) fn native_set_flying_speed(&mut self, player: u128, speed: Option<f32>) -> bool {
        let speed = speed
            .filter(|s| s.is_finite())
            .map_or(FLYING_VANILLA, |s| s.clamp(0.0, FLYING_MAX));
        let Some(host) = self.lan.as_mut() else {
            return false;
        };
        let Some(conn) = host
            .hub
            .guests
            .iter()
            .find(|g| g.uuid == player && g.placed)
            .map(|g| g.conn)
        else {
            return false;
        };
        host.hub.set_flying_speed(conn, speed);
        true
    }
}

#[cfg(test)]
mod motion_tests {
    use super::*;

    #[test]
    fn a_push_adds_to_the_velocity_clamped() {
        assert_eq!(
            pushed([0.1, -0.08, 0.0], [0.8, 0.55, -0.5]),
            Some([0.1 + 0.8f32 as f64, -0.08 + 0.55f32 as f64, -0.5])
        );
        assert_eq!(pushed([0.0; 3], [9.0, -9.0, 0.0]), Some([4.0, -4.0, 0.0]));
        assert_eq!(pushed([1.0; 3], [f32::NAN, 0.0, 0.0]), None);
        assert_eq!(pushed([1.0; 3], [0.0, f32::INFINITY, 0.0]), None);
    }
}
