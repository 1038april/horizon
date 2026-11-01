//! OptiFine connected textures (`optifine/ctm`) at mesh time. The active packs' rules are read
//! once at load, their tiles become sprite layers, and each block state and sprite layer gets
//! the short list of rules that can touch it; meshing a face then walks only those lists
//! ([`ConnectedTextures::pick`]). Without CTM files there are no rules and nothing changes.
//!
//! The rules and the tile geometry are `horizon_optifine::ctm`'s.

use crate::sprites::Sprites;
use horizon_optifine::ctm::{
    self, Connect, Method, Orient, Rule, Tile,
    connect::{self as geometry, Axes, Axis, Face},
};
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::{collections::HashMap, path::Path};

/// A tile as the mesher draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileLayer {
    Layer {
        layer: u16,
        /// The sprite has fractional alpha (drawn in the translucent pass).
        translucent: bool,
        /// Every texel is opaque (drawn without an alpha test).
        opaque: bool,
    },
    /// Try the next rule.
    Skip,
    /// The face's own sprite.
    Default,
}

/// What a face draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Its own sprite.
    Keep,
    Tile(TileLayer),
    /// `ctm_compact`: a tile per quarter of the sprite (top-left, top-right, bottom-left,
    /// bottom-right).
    Quarters([TileLayer; 4]),
}

struct LoadedRule {
    rule: Rule,
    tiles: Vec<TileLayer>,
}

#[derive(Clone, Copy, Default)]
struct StateInfo {
    /// Index of the state's block (`connect=block`).
    block: u32,
    axis: Option<Axis>,
    /// `half=upper`: `linked` random tiles follow the lower half.
    upper: bool,
}

#[derive(Default)]
pub struct ConnectedTextures {
    rules: Vec<LoadedRule>,
    /// By block state, then by sprite layer: ranges of `lists` (rule indices, in rule order).
    by_state: Vec<(u32, u32)>,
    by_layer: Vec<(u32, u32)>,
    lists: Vec<u16>,
    states: Vec<StateInfo>,
    /// By block state: the sprite of its model's face on each side (`connect=tile`); empty
    /// when no rule connects by tile.
    face_layers: Vec<[u16; 6]>,
}

/// The sprite name of a texture resource path: `ns:path` under `textures/`, else the path
/// itself (as [`Sprites::insert_file`] names a tile).
fn sprite_name(resource: &str) -> String {
    let parts = resource
        .strip_prefix("assets/")
        .and_then(|p| p.split_once('/'))
        .and_then(|(ns, p)| Some((ns, p.strip_prefix("textures/")?.strip_suffix(".png")?)));
    match parts {
        Some((ns, path)) => format!("{ns}:{path}"),
        None => resource.to_owned(),
    }
}

impl ConnectedTextures {
    /// The rules under `root`, with their tiles inserted into `sprites` (before it finishes).
    /// `face_layer` answers a block state's sprite on a side, for rules that connect by tile.
    pub fn load(
        root: &Path,
        registry: &BlockRegistry,
        sprites: &mut Sprites,
        mut face_layer: impl FnMut(BlockStateId, Face) -> Option<u32>,
    ) -> Self {
        let mut rules = ctm::load(root);
        // Rule indices are `u16`.
        rules.truncate(usize::from(u16::MAX));
        if rules.is_empty() {
            return Self::default();
        }
        let loaded = rules
            .into_iter()
            .filter_map(|rule| {
                let tiles = rule
                    .tiles
                    .iter()
                    .map(|tile| match tile {
                        Tile::Skip => Ok(TileLayer::Skip),
                        Tile::Default => Ok(TileLayer::Default),
                        Tile::Texture(path) => {
                            let layer = sprites.insert_file(root, path)?;
                            Ok(TileLayer::Layer {
                                layer: u16::try_from(layer)?,
                                translucent: sprites.translucent[layer as usize],
                                opaque: sprites.opaque[layer as usize],
                            })
                        }
                    })
                    .collect::<crate::Result<Vec<_>>>();
                match tiles {
                    Ok(tiles) => Some(LoadedRule { rule, tiles }),
                    Err(e) => {
                        log::warn!("Horizon: connected textures {}: a tile: {e}", rule.path);
                        None
                    }
                }
            })
            .collect();
        let states = (0..registry.len() as u32).map(|id| {
            registry
                .state(BlockStateId(id))
                .map(|s| (s.name.as_str(), &s.properties))
        });
        let mut out = Self::build(loaded, states, |name| sprites.layers.get(name).copied());
        if out.rules.iter().any(|r| r.rule.connect == Connect::Tile) {
            out.face_layers = (0..registry.len() as u32)
                .map(|id| {
                    Face::ALL
                        .map(|f| face_layer(BlockStateId(id), f).map_or(u16::MAX, |l| l as u16))
                })
                .collect();
        }
        out
    }

    /// The lookups for `rules` (already ordered) over every block state (`None` for an id the
    /// registry skips), `layer` answering a sprite name's layer.
    fn build<'a>(
        rules: Vec<LoadedRule>,
        states: impl Iterator<Item = Option<(&'a str, &'a std::collections::BTreeMap<String, String>)>>,
        layer: impl Fn(&str) -> Option<u32>,
    ) -> Self {
        let mut out = Self {
            rules,
            ..Self::default()
        };
        let mut blocks = HashMap::new();
        let mut by_layer: Vec<Vec<u16>> = Vec::new();
        for (i, r) in out.rules.iter().enumerate() {
            for tile in &r.rule.match_tiles {
                if let Some(l) = layer(&sprite_name(tile)) {
                    let l = l as usize;
                    if by_layer.len() <= l {
                        by_layer.resize(l + 1, Vec::new());
                    }
                    by_layer[l].push(i as u16);
                }
            }
        }
        for list in by_layer {
            out.by_layer.push(out.push_list(&list));
        }
        let mut list = Vec::new();
        for state in states {
            let Some((name, props)) = state else {
                out.by_state.push((0, 0));
                out.states.push(StateInfo::default());
                continue;
            };
            let next = blocks.len() as u32;
            let block = *blocks.entry(name).or_insert(next);
            let prop = |k: &str| props.get(k).map(String::as_str);
            out.states.push(StateInfo {
                block,
                axis: match prop("axis") {
                    Some("x") => Some(Axis::X),
                    Some("y") => Some(Axis::Y),
                    Some("z") => Some(Axis::Z),
                    _ => None,
                },
                upper: prop("half") == Some("upper"),
            });
            list.clear();
            list.extend(
                (0..out.rules.len())
                    .filter(|&i| out.rules[i].rule.matches_block(name, prop))
                    .map(|i| i as u16),
            );
            let range = out.push_list(&list);
            out.by_state.push(range);
        }
        out
    }

    fn push_list(&mut self, list: &[u16]) -> (u32, u32) {
        let start = self.lists.len() as u32;
        self.lists.extend_from_slice(list);
        (start, list.len() as u32)
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    fn list(&self, ranges: &[(u32, u32)], i: usize) -> &[u16] {
        ranges.get(i).map_or(&[], |&(start, len)| {
            &self.lists[start as usize..(start + len) as usize]
        })
    }

    /// What the `face` of the block `state` at `pos` draws, for a quad with sprite `layer`.
    /// `neighbour` answers the state at an offset from the block; `biome` its biome, asked only
    /// by rules that test one.
    pub fn pick(
        &self,
        state: u32,
        layer: u16,
        face: Face,
        pos: [i32; 3],
        neighbour: impl Fn([i32; 3]) -> u32,
        biome: impl Fn() -> Option<String>,
    ) -> Pick {
        let (by_state, by_layer) = (
            self.list(&self.by_state, state as usize),
            self.list(&self.by_layer, layer as usize),
        );
        // Both lists are in rule order: walk them as one.
        let (mut a, mut b) = (by_state.iter().peekable(), by_layer.iter().peekable());
        loop {
            let next = match (a.peek(), b.peek()) {
                (Some(&&x), Some(&&y)) if x == y => {
                    a.next();
                    b.next()
                }
                (Some(&&x), Some(&&y)) if x < y => a.next(),
                (Some(_), None) => a.next(),
                (_, Some(_)) => b.next(),
                (None, None) => return Pick::Keep,
            };
            let Some(&i) = next else {
                return Pick::Keep;
            };
            let r = &self.rules[i as usize];
            if let Some(pick) = self.apply(r, state, layer, face, pos, &neighbour, &biome) {
                return pick;
            }
        }
    }

    /// One rule on a face: `None` when it doesn't apply or picks `<skip>`.
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &self,
        r: &LoadedRule,
        state: u32,
        layer: u16,
        face: Face,
        pos: [i32; 3],
        neighbour: &impl Fn([i32; 3]) -> u32,
        biome: &impl Fn() -> Option<String>,
    ) -> Option<Pick> {
        let rule = &r.rule;
        if rule.faces & face.bit() == 0
            || rule.heights.as_ref().is_some_and(|h| !h.contains(pos[1]))
            // Section meshes see no block entity, so no custom name.
            || rule.name.is_some()
            || rule.method.is_overlay()
        {
            return None;
        }
        if let Some(biomes) = &rule.biomes
            && !biome().is_some_and(|b| biomes.matches(&b))
        {
            return None;
        }
        let info = self.states.get(state as usize).copied().unwrap_or_default();
        let joins = |offset: [i32; 3]| {
            let other = neighbour(offset);
            match rule.connect {
                Connect::Block => {
                    self.states.get(other as usize).map(|s| s.block) == Some(info.block)
                }
                Connect::State => other == state,
                Connect::Tile => self
                    .face_layers
                    .get(other as usize)
                    .is_some_and(|f| f[face as usize] == layer),
            }
        };
        let axes = |face| match (rule.orient, info.axis) {
            (Orient::StateAxis, Some(axis)) => Axes::column(face, axis),
            // `orient=texture` follows the quad's own UVs; vanilla cubes are what it is used on.
            _ => Axes::cube(face),
        };
        let mask = || geometry::neighbours(axes(face), face.normal(), rule.inner_seams, joins);
        let tile = |i: usize| r.tiles.get(i).copied().unwrap_or(TileLayer::Default);
        let index = match rule.method {
            Method::Ctm => geometry::ctm_index(mask()),
            Method::CtmCompact => {
                let mask = mask();
                let case = geometry::ctm_index(mask) as u8;
                match rule.compact_overrides.iter().find(|&&(c, _)| c == case) {
                    Some(&(_, t)) => t,
                    None => {
                        let quarters = geometry::compact_quarters(mask).map(|q| tile(q.into()));
                        return if quarters.contains(&TileLayer::Skip) {
                            None
                        } else {
                            Some(Pick::Quarters(quarters))
                        };
                    }
                }
            }
            Method::Horizontal => geometry::horizontal(mask()),
            Method::Vertical => geometry::vertical(mask()),
            Method::HorizontalVertical => geometry::horizontal_vertical(mask()),
            Method::VerticalHorizontal => geometry::vertical_horizontal(mask()),
            Method::Top => {
                if matches!(face, Face::Up | Face::Down) || !joins([0, 1, 0]) {
                    return Some(Pick::Keep);
                }
                0
            }
            Method::Fixed => 0,
            Method::Random => {
                let face = geometry::symmetric_face(face, rule.symmetry);
                let pos = if rule.linked && info.upper {
                    [pos[0], pos[1] - 1, pos[2]]
                } else {
                    pos
                };
                geometry::random_index(pos, face, rule.random_loops, &rule.weights, r.tiles.len())
            }
            Method::Repeat => {
                let face = geometry::symmetric_face(face, rule.symmetry);
                geometry::repeat_index(pos, axes(face), rule.width, rule.height)
            }
            _ => return None,
        };
        match tile(index) {
            TileLayer::Skip => None,
            TileLayer::Default => Some(Pick::Keep),
            t => Some(Pick::Tile(t)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_optifine::Properties;
    use std::collections::BTreeMap;

    /// A tile whose layer says which tile of its rule it is: rule `r`'s tile `i` is layer
    /// `1000 * (r + 1) + i`.
    fn rules(files: &[(&str, &str)]) -> Vec<LoadedRule> {
        let mut rules: Vec<Rule> = files
            .iter()
            .map(|(path, text)| {
                Rule::parse(
                    &format!("assets/minecraft/optifine/ctm/{path}"),
                    &Properties::parse(text),
                )
                .unwrap()
                .unwrap()
            })
            .collect();
        ctm::sort(&mut rules);
        rules
            .into_iter()
            .enumerate()
            .map(|(n, rule)| LoadedRule {
                tiles: rule
                    .tiles
                    .iter()
                    .enumerate()
                    .map(|(i, t)| match t {
                        Tile::Skip => TileLayer::Skip,
                        Tile::Default => TileLayer::Default,
                        Tile::Texture(_) => TileLayer::Layer {
                            layer: (1000 * (n + 1) + i) as u16,
                            translucent: false,
                            opaque: false,
                        },
                    })
                    .collect(),
                rule,
            })
            .collect()
    }

    /// State 0 air, 1 glass, 2 stone, 3 and 4 oak logs (axis y, x), 5 and 6 a door (lower,
    /// upper); the stone sprite is layer 7.
    fn world(files: &[(&str, &str)]) -> ConnectedTextures {
        let props = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        let states = [
            ("minecraft:air", props(&[])),
            ("minecraft:glass", props(&[])),
            ("minecraft:stone", props(&[])),
            ("minecraft:oak_log", props(&[("axis", "y")])),
            ("minecraft:oak_log", props(&[("axis", "x")])),
            ("minecraft:oak_door", props(&[("half", "lower")])),
            ("minecraft:oak_door", props(&[("half", "upper")])),
        ];
        ConnectedTextures::build(
            rules(files),
            states.iter().map(|(n, p)| Some((*n, p))),
            |name| (name == "minecraft:block/stone").then_some(7),
        )
    }

    fn tile(layer: u16) -> Pick {
        Pick::Tile(TileLayer::Layer {
            layer,
            translucent: false,
            opaque: false,
        })
    }

    /// `pick` on `face` of the block at the origin among `blocks` (offset, state).
    fn pick_in(
        ctm: &ConnectedTextures,
        state: u32,
        layer: u16,
        face: Face,
        blocks: &[([i32; 3], u32)],
    ) -> Pick {
        let at = |o: [i32; 3]| blocks.iter().find(|(p, _)| *p == o).map_or(0, |b| b.1);
        ctm.pick(state, layer, face, [0, 64, 0], at, || None)
    }

    const GLASS: &str = "matchBlocks=glass\nmethod=ctm\ntiles=0-46";

    #[test]
    fn glass_joins_its_neighbours() {
        let ctm = world(&[("glass.properties", GLASS)]);
        // Alone: tile 0. A north face's right is west: glass there is the left end of a row.
        assert_eq!(pick_in(&ctm, 1, 3, Face::North, &[]), tile(1000));
        assert_eq!(
            pick_in(&ctm, 1, 3, Face::North, &[([-1, 0, 0], 1)]),
            tile(1001)
        );
        // Glass all around in the face's plane: the middle tile, 26.
        let ring: Vec<_> = [-1, 0, 1]
            .into_iter()
            .flat_map(|x| [-1, 0, 1].map(|y| ([x, y, 0], 1)))
            .collect();
        assert_eq!(pick_in(&ctm, 1, 3, Face::North, &ring), tile(1026));
        // Stone beside it doesn't join; stone has no rule.
        assert_eq!(
            pick_in(&ctm, 1, 3, Face::North, &[([-1, 0, 0], 2)]),
            tile(1000)
        );
        assert_eq!(pick_in(&ctm, 2, 7, Face::North, &[]), Pick::Keep);
    }

    #[test]
    fn rule_order_faces_and_skips() {
        let ctm = world(&[
            (
                "a_sides.properties",
                "matchBlocks=stone\nmethod=fixed\ntiles=<skip>\nfaces=sides",
            ),
            ("b_glass.properties", GLASS),
            ("stone.properties", "method=fixed\ntiles=x\nfaces=top"),
            ("z.properties", "matchBlocks=stone\nmethod=fixed\ntiles=y"),
        ]);
        // The tile rule (stone.properties, by its file name) comes first, on the top face only.
        assert_eq!(pick_in(&ctm, 2, 7, Face::Up, &[]), tile(1000));
        // Sides: a_sides skips, so z draws.
        assert_eq!(pick_in(&ctm, 2, 7, Face::North, &[]), tile(4000));
        // The tile rule follows the sprite, whatever the block: glass with the stone sprite.
        assert_eq!(pick_in(&ctm, 1, 7, Face::Up, &[]), tile(1000));
    }

    #[test]
    fn compact_quarters_and_overrides() {
        let ctm = world(&[(
            "glass.properties",
            "matchBlocks=glass\nmethod=ctm_compact\ntiles=0-5\nctm.1=5",
        )]);
        let layer = |i| TileLayer::Layer {
            layer: 1000 + i,
            translucent: false,
            opaque: false,
        };
        assert_eq!(
            pick_in(&ctm, 1, 3, Face::North, &[]),
            Pick::Quarters([layer(0); 4])
        );
        // Joined right only is case 1, replaced whole by tile 5.
        assert_eq!(
            pick_in(&ctm, 1, 3, Face::North, &[([-1, 0, 0], 1)]),
            tile(1005)
        );
        // Joined left only (east of a north face): quarters [3, 0, 3, 0].
        assert_eq!(
            pick_in(&ctm, 1, 3, Face::North, &[([1, 0, 0], 1)]),
            Pick::Quarters([layer(3), layer(0), layer(3), layer(0)])
        );
    }

    #[test]
    fn logs_turn_with_their_axis() {
        let ctm = world(&[(
            "log.properties",
            "matchBlocks=oak_log\nmethod=vertical\ntiles=0-3",
        )]);
        // An upright log with a log above: the bottom of a column.
        assert_eq!(
            pick_in(&ctm, 3, 3, Face::North, &[([0, 1, 0], 3)]),
            tile(1000)
        );
        // A log lying along x: on its north face the texture's up points east, so a log to the
        // east (same block, other axis state) is above it.
        assert_eq!(
            pick_in(&ctm, 4, 3, Face::North, &[([1, 0, 0], 4)]),
            tile(1000)
        );
        assert_eq!(
            pick_in(&ctm, 4, 3, Face::North, &[([0, 1, 0], 4)]),
            tile(1003)
        );
    }

    #[test]
    fn top_and_connect_state() {
        let ctm = world(&[(
            "block_oak_log.properties",
            "method=top\ntiles=0\nconnect=state",
        )]);
        assert_eq!(
            pick_in(&ctm, 3, 3, Face::North, &[([0, 1, 0], 3)]),
            tile(1000)
        );
        // The other axis is another state.
        assert_eq!(
            pick_in(&ctm, 3, 3, Face::North, &[([0, 1, 0], 4)]),
            Pick::Keep
        );
        assert_eq!(pick_in(&ctm, 3, 3, Face::Up, &[([0, 1, 0], 3)]), Pick::Keep);
    }

    #[test]
    fn linked_random_follows_the_lower_half() {
        let ctm = world(&[(
            "door.properties",
            "matchBlocks=oak_door\nmethod=random\ntiles=0-7\nlinked=true",
        )]);
        for x in 0..20 {
            let lower = ctm.pick(5, 3, Face::North, [x, 64, 0], |_| 0, || None);
            let upper = ctm.pick(6, 3, Face::North, [x, 65, 0], |_| 0, || None);
            assert_eq!(lower, upper, "{x}");
        }
    }

    #[test]
    fn heights_and_biomes() {
        let ctm = world(&[(
            "glass.properties",
            "matchBlocks=glass\nmethod=fixed\ntiles=0\nheights=0-63\nbiomes=desert",
        )]);
        let at = |y, biome: &'static str| {
            ctm.pick(1, 3, Face::Up, [0, y, 0], |_| 0, || Some(biome.into()))
        };
        assert_eq!(at(10, "minecraft:desert"), tile(1000));
        assert_eq!(at(64, "minecraft:desert"), Pick::Keep);
        assert_eq!(at(10, "minecraft:plains"), Pick::Keep);
    }

    #[test]
    fn sprite_names() {
        assert_eq!(
            sprite_name("assets/minecraft/textures/block/dirt.png"),
            "minecraft:block/dirt"
        );
        assert_eq!(
            sprite_name("assets/minecraft/optifine/ctm/glass/0.png"),
            "assets/minecraft/optifine/ctm/glass/0.png"
        );
    }
}
