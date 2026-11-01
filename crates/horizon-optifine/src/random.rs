//! Random entity textures (`optifine/random/entity/<path>.properties`, the legacy `mob/` folder):
//! the numbered rules that pick which of a texture's numbered variants an entity wears.
//!
//! A rule holds `textures.<n>` (or `skins.<n>`), the variant indices it chooses from (1 is the
//! default texture), their optional `weights.<n>`, and conditions on the entity. Rules are tried
//! in order of `<n>`; the first whose conditions all hold picks one of its textures from the
//! entity's seed, so the same entity keeps the same texture. No rule matching means the default.
//!
//! Conditions: `biomes`, `heights` (and the legacy `minHeight`/`maxHeight`), `name` (and ETF's
//! `names`), `professions`, `colors` (`collarColors`), `baby`, `health` (in points or percent),
//! `moonPhase`, `dayTime`, `weather`, `sizes` and `blocks`. The spawn-time ones (biome, height,
//! block, moon, time, weather) are the caller's to fix when the entity is first seen; the others
//! can change, and [`Rules::updates`] says whether a rule file reads any of them.
//!
//! Written from OptiFine's `random_entities.properties` and ETF's `RANDOM_GUIDE.md` and
//! `random_entities.properties` docs; no mod's source was read.

use crate::matchers::{BlockMatch, IdList, Name, RangeList, namespaced};
use crate::properties::Properties;

/// The most variants one rule lists (`textures.1=1-100000` stays small).
const MAX_TEXTURES: usize = 256;

/// What a rule can ask about an entity. `None` is a fact the entity doesn't have (a creeper's
/// profession): a rule asking for it doesn't match.
#[derive(Clone, Debug, Default)]
pub struct Facts<'a> {
    /// Where the entity was first seen: its biome (`minecraft:plains`) and block height.
    pub biome: Option<&'a str>,
    pub y: Option<i32>,
    /// The block it stood on and the block it stood in when first seen (ETF checks both).
    pub blocks: &'a [Block<'a>],
    /// The plain text of its custom name.
    pub name: Option<&'a str>,
    /// Villager profession (`minecraft:farmer`) and level.
    pub profession: Option<(&'a str, i32)>,
    /// Collar, wool, carpet or shell colour, as a dye name (`light_blue`).
    pub color: Option<&'a str>,
    pub baby: Option<bool>,
    /// Health and max health.
    pub health: Option<(f32, f32)>,
    /// When first seen: moon phase (0-7), time of day (0-23999) and weather.
    pub moon_phase: Option<i32>,
    pub day_time: Option<i32>,
    pub weather: Option<Weather>,
    /// Slime, magma cube or phantom size.
    pub size: Option<i32>,
}

/// A block and the values of its state properties.
#[derive(Clone, Copy, Debug)]
pub struct Block<'a> {
    pub id: &'a str,
    pub states: &'a [(&'a str, &'a str)],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Thunder,
}

/// One `.properties` file's rules, in order.
#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Default)]
pub struct Rule {
    /// Variant indices; 1 is the default texture.
    pub textures: Vec<u32>,
    /// Same length as `textures`, or empty for an even choice.
    pub weights: Vec<u32>,
    /// ETF's `seedOffset`: added to the entity's seed.
    pub seed_offset: i32,
    biomes: Option<IdList>,
    heights: Option<RangeList>,
    names: Vec<(Name, bool)>,
    professions: Vec<(String, Option<RangeList>)>,
    colors: Option<Vec<String>>,
    baby: Option<bool>,
    health: Option<(RangeList, bool)>,
    moon_phase: Option<RangeList>,
    day_time: Option<RangeList>,
    weather: Option<Vec<Weather>>,
    sizes: Option<RangeList>,
    blocks: Vec<(BlockMatch, bool)>,
}

impl Rules {
    pub fn parse(props: &Properties) -> Self {
        let mut numbers: Vec<u32> = ["textures.", "skins."]
            .iter()
            .flat_map(|p| props.with_prefix(p))
            .filter_map(|(n, _)| n.parse().ok())
            .collect();
        numbers.sort_unstable();
        numbers.dedup();
        let rules = numbers
            .into_iter()
            .filter_map(|n| Rule::parse(props, n))
            .collect();
        Self { rules }
    }

    /// The variant this entity wears: the first matching rule's pick, or 1 (the default).
    pub fn select(&self, facts: &Facts, seed: i32) -> u32 {
        self.rules
            .iter()
            .find(|r| r.matches(facts))
            .map_or(1, |r| r.pick(seed))
    }

    /// Whether any rule reads a fact that changes while the entity lives (name, profession,
    /// colour, baby, health), so the choice should be checked again now and then.
    pub fn updates(&self) -> bool {
        self.rules.iter().any(|r| {
            !r.names.is_empty()
                || !r.professions.is_empty()
                || r.colors.is_some()
                || r.baby.is_some()
                || r.health.is_some()
        })
    }

    /// The highest variant index any rule lists.
    pub fn max_texture(&self) -> u32 {
        self.rules
            .iter()
            .flat_map(|r| r.textures.iter().copied())
            .max()
            .unwrap_or(1)
    }
}

impl Rule {
    fn parse(props: &Properties, n: u32) -> Option<Self> {
        let get = |key: &str| props.get(&format!("{key}.{n}"));
        let textures = expand(&RangeList::parse(get("textures").or(get("skins"))?)?);
        if textures.is_empty() {
            return None;
        }
        let weights = get("weights")
            .and_then(RangeList::parse)
            .map(|w| w.0.iter().map(|&(lo, _)| lo.max(0) as u32).collect())
            .filter(|w: &Vec<u32>| w.len() == textures.len() && w.iter().any(|&w| w > 0))
            .unwrap_or_default();
        let ranges = |key| get(key).and_then(RangeList::parse);
        let heights = ranges("heights").or_else(|| {
            let min = get("minHeight").and_then(|v| v.parse().ok());
            let max = get("maxHeight").and_then(|v| v.parse().ok());
            (min.is_some() || max.is_some())
                .then(|| RangeList(vec![(min.unwrap_or(i32::MIN), max.unwrap_or(i32::MAX))]))
        });
        let mut names: Vec<(Name, bool)> =
            get("name").and_then(negatable_name).into_iter().collect();
        names.extend(get("names").map_or(Vec::new(), |v| {
            quoted_words(v).filter_map(negatable_name).collect()
        }));
        Some(Self {
            textures,
            weights,
            seed_offset: get("seedOffset").and_then(|v| v.parse().ok()).unwrap_or(0),
            biomes: get("biomes").map(IdList::parse),
            heights,
            names,
            professions: get("professions").map_or(Vec::new(), parse_professions),
            colors: get("colors")
                .or(get("collarColors"))
                .map(|v| v.split_whitespace().map(str::to_ascii_lowercase).collect()),
            baby: get("baby").and_then(|v| v.parse().ok()),
            health: get("health").and_then(|v| {
                let percent = v.contains('%');
                Some((RangeList::parse(&v.replace('%', ""))?, percent))
            }),
            moon_phase: ranges("moonPhase"),
            day_time: ranges("dayTime"),
            weather: get("weather").map(|v| {
                v.split_whitespace()
                    .filter_map(|w| match w {
                        "clear" => Some(Weather::Clear),
                        "rain" => Some(Weather::Rain),
                        "thunder" => Some(Weather::Thunder),
                        _ => None,
                    })
                    .collect()
            }),
            sizes: ranges("sizes"),
            blocks: get("blocks").map_or(Vec::new(), |v| {
                v.split_whitespace()
                    .filter_map(|b| {
                        let (b, negated) = b.strip_prefix('!').map_or((b, false), |b| (b, true));
                        Some((BlockMatch::parse(b)?, negated))
                    })
                    .collect()
            }),
        })
    }

    pub fn matches(&self, f: &Facts) -> bool {
        let ranged = |list: &Option<RangeList>, fact: Option<i32>| {
            list.as_ref()
                .is_none_or(|l| fact.is_some_and(|v| l.contains(v)))
        };
        self.biomes
            .as_ref()
            .is_none_or(|l| f.biome.is_some_and(|b| l.matches(b)))
            && ranged(&self.heights, f.y)
            && (self.names.is_empty() || {
                let name = f.name.unwrap_or("");
                self.names
                    .iter()
                    .any(|(n, negated)| n.matches(name) != *negated)
            })
            && (self.professions.is_empty()
                || f.profession.is_some_and(|(p, level)| {
                    let p = namespaced(p);
                    self.professions.iter().any(|(want, levels)| {
                        *want == p && levels.as_ref().is_none_or(|l| l.contains(level))
                    })
                }))
            && self
                .colors
                .as_ref()
                .is_none_or(|c| f.color.is_some_and(|f| c.iter().any(|c| c == f)))
            && self.baby.is_none_or(|b| f.baby == Some(b))
            && self.health.as_ref().is_none_or(|(l, percent)| {
                f.health.is_some_and(|(health, max)| {
                    let value = if *percent {
                        health / max.max(f32::MIN_POSITIVE) * 100.0
                    } else {
                        health
                    };
                    l.contains(value as i32)
                })
            })
            && ranged(&self.moon_phase, f.moon_phase)
            && ranged(&self.day_time, f.day_time)
            && self
                .weather
                .as_ref()
                .is_none_or(|w| f.weather.is_some_and(|f| w.contains(&f)))
            && ranged(&self.sizes, f.size)
            && (self.blocks.is_empty() || block_matches(&self.blocks, f.blocks))
    }

    /// One of the rule's textures, by weight, from the entity's seed.
    pub fn pick(&self, seed: i32) -> u32 {
        let r = seed.wrapping_add(self.seed_offset).unsigned_abs();
        if self.weights.is_empty() {
            return self.textures[r as usize % self.textures.len()];
        }
        let total: u64 = self.weights.iter().map(|&w| u64::from(w)).sum();
        let mut left = u64::from(r) % total;
        for (&texture, &weight) in self.textures.iter().zip(&self.weights) {
            if left < u64::from(weight) {
                return texture;
            }
            left -= u64::from(weight);
        }
        self.textures[0]
    }
}

/// An entity's seed from its UUID (most then least significant 64 bits): Java's
/// `UUID.hashCode`, so it is fixed for the entity's whole life, on every client.
pub fn seed(most: i64, least: i64) -> i32 {
    let x = most ^ least;
    ((x >> 32) ^ x) as i32
}

fn expand(list: &RangeList) -> Vec<u32> {
    let mut out = Vec::new();
    for &(lo, hi) in &list.0 {
        for i in lo.max(0)..=hi.min(lo.max(0).saturating_add(MAX_TEXTURES as i32)) {
            if out.len() == MAX_TEXTURES {
                return out;
            }
            out.push(i as u32);
        }
    }
    out
}

fn negatable_name(value: &str) -> Option<(Name, bool)> {
    match value.strip_prefix('!') {
        Some(v) => Some((Name::parse(v)?, true)),
        None => Some((Name::parse(value)?, false)),
    }
}

/// Words split on spaces, a `"double quoted"` group kept as one word (ETF's `names`).
fn quoted_words(value: &str) -> impl Iterator<Item = &str> {
    let mut rest = value.trim_start();
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let (word, after) = match rest.strip_prefix('"') {
            Some(q) => q.split_once('"').unwrap_or((q, "")),
            None => rest.split_once(' ').unwrap_or((rest, "")),
        };
        rest = after.trim_start();
        Some(word)
    })
}

/// `farmer librarian:1,3-4 minecraft:fisherman mymod:mayor:2`.
fn parse_professions(value: &str) -> Vec<(String, Option<RangeList>)> {
    value
        .split_whitespace()
        .map(|entry| match entry.rsplit_once(':') {
            Some((name, levels)) if levels.starts_with(|c: char| c.is_ascii_digit()) => {
                (namespaced(name), RangeList::parse(levels))
            }
            _ => (namespaced(entry), None),
        })
        .collect()
}

/// Any listed block (not negated) is one of the entity's, and no negated one is.
fn block_matches(list: &[(BlockMatch, bool)], blocks: &[Block]) -> bool {
    let hit = |m: &BlockMatch| {
        blocks.iter().any(|b| {
            m.matches(b.id, |key| {
                b.states.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
            })
        })
    };
    let mut wanted = list.iter().filter(|(_, negated)| !negated).peekable();
    let any_wanted = wanted.peek().is_none() || wanted.any(|(m, _)| hit(m));
    any_wanted && !list.iter().any(|(m, negated)| *negated && hit(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> Rules {
        Rules::parse(&Properties::parse(text))
    }

    #[test]
    fn first_matching_rule_wins_and_weights_split_seeds() {
        // OptiFine's own creeper example.
        let r = rules(
            "textures.1=10-14\nweights.1=10 10 10 3 10\nheights.1=0-55\n\
             textures.2=5 7 9\nbiomes.2=desert forest\nheights.2=80-255\n\
             textures.3=1-4 6 8 15-20\n",
        );
        assert_eq!(r.rules.len(), 3);
        assert_eq!(r.max_texture(), 20);
        assert!(!r.updates());
        let underground = Facts {
            y: Some(30),
            biome: Some("minecraft:plains"),
            ..Default::default()
        };
        // Weights 10 10 10 3 10 out of 43: seed % 43 in 30..33 is the fourth texture.
        assert_eq!(r.select(&underground, 0), 10);
        assert_eq!(r.select(&underground, 31), 13);
        assert_eq!(r.select(&underground, 42), 14);
        assert_eq!(r.select(&underground, 43 + 25), 12);
        let hills = Facts {
            y: Some(100),
            biome: Some("minecraft:desert"),
            ..Default::default()
        };
        assert_eq!(r.select(&hills, 4), 7, "4 % 3 = 1: the second of 5 7 9");
        // Plains at 100 falls to rule 3's 12 textures: 1 2 3 4 6 8 15..20.
        let plains = Facts {
            biome: Some("plains"),
            ..hills.clone()
        };
        assert_eq!(r.select(&plains, 5), 8);
        assert_eq!(r.select(&plains, -6), 15, "|-6| % 12 = 6");
        // No facts at all: height-bound rules fail, the fallback still applies.
        assert_eq!(r.select(&Facts::default(), 1), 2);
    }

    #[test]
    fn no_rule_matching_is_the_default() {
        let r = rules("skins.1=2 3\nbaby.1=true\n");
        assert!(r.updates());
        assert_eq!(r.select(&Facts::default(), 1), 1);
        let baby = Facts {
            baby: Some(true),
            ..Default::default()
        };
        assert_eq!(r.select(&baby, 1), 3);
    }

    #[test]
    fn conditions() {
        let check = |text: &str, facts: Facts| rules(text).select(&facts, 0) == 2;
        let named = |name| Facts {
            name: Some(name),
            ..Default::default()
        };
        assert!(check("textures.1=2\nname.1=ipattern:pink*", named("Pinky")));
        assert!(!check("textures.1=2\nname.1=!Pinky", named("Pinky")));
        assert!(check("textures.1=2\nname.1=!Pinky", named("Bob")));
        assert!(check(
            "textures.1=2\nnames.1=Bob \"John Smith\"",
            named("John Smith")
        ));
        assert!(!check(
            "textures.1=2\nnames.1=Bob \"John Smith\"",
            named("John")
        ));

        let villager = |p, level| Facts {
            profession: Some((p, level)),
            ..Default::default()
        };
        let prof = "textures.1=2\nprofessions.1=farmer minecraft:librarian:1,3-4 mymod:mayor:2";
        assert!(check(prof, villager("minecraft:farmer", 5)));
        assert!(check(prof, villager("librarian", 3)));
        assert!(!check(prof, villager("librarian", 2)));
        assert!(check(prof, villager("mymod:mayor", 2)));
        assert!(!check(prof, Facts::default()));

        let hurt = |health| Facts {
            health: Some((health, 40.0)),
            ..Default::default()
        };
        assert!(check("textures.1=2\nhealth.1=0-50%", hurt(20.0)));
        assert!(!check("textures.1=2\nhealth.1=0-50%", hurt(21.0)));
        assert!(check("textures.1=2\nhealth.1=5-8 10-12", hurt(11.5)));
        assert!(!check("textures.1=2\nhealth.1=5-8 10-12", hurt(9.0)));

        let colored = |c| Facts {
            color: Some(c),
            ..Default::default()
        };
        assert!(check(
            "textures.1=2\ncollarColors.1=pink magenta",
            colored("pink")
        ));
        assert!(!check(
            "textures.1=2\ncolors.1=pink magenta",
            colored("red")
        ));

        let world = Facts {
            moon_phase: Some(3),
            day_time: Some(18500),
            weather: Some(Weather::Thunder),
            size: Some(3),
            y: Some(-40),
            ..Default::default()
        };
        assert!(check(
            "textures.1=2\nmoonPhase.1=0-3\ndayTime.1=0-1000 18000-24000\n\
             weather.1=rain thunder\nsizes.1=3\nheights.1=(-64)-(-30)",
            world.clone()
        ));
        assert!(!check("textures.1=2\nweather.1=clear", world.clone()));
        assert!(check(
            "textures.1=2\nminHeight.1=-50\nmaxHeight.1=-40",
            world.clone()
        ));
        assert!(!check("textures.1=2\nminHeight.1=-39", world));
    }

    #[test]
    fn blocks() {
        let r = rules("textures.1=2\nblocks.1=sand oak_log:axis=y\ntextures.2=3\nblocks.2=!stone");
        let pick = |blocks: &[Block]| {
            r.select(
                &Facts {
                    blocks,
                    ..Default::default()
                },
                0,
            )
        };
        let block = |id| Block { id, states: &[] };
        assert_eq!(pick(&[block("minecraft:sand"), block("minecraft:air")]), 2);
        let upright = [Block {
            id: "minecraft:oak_log",
            states: &[("axis", "y")],
        }];
        assert_eq!(pick(&upright), 2);
        let sideways = [Block {
            id: "minecraft:oak_log",
            states: &[("axis", "x")],
        }];
        assert_eq!(pick(&sideways), 3, "rule 2: anything but stone");
        assert_eq!(pick(&[block("minecraft:stone")]), 1);
    }

    #[test]
    fn broken_rules_are_skipped() {
        let r = rules(
            "textures.1=abc\nbiomes.1=desert\n\
             textures.2=2 3\nweights.2=5\n\
             skins.7=1-100000\n",
        );
        assert_eq!(r.rules.len(), 2, "rule 1 lists no textures");
        assert!(r.rules[0].weights.is_empty(), "one weight for two textures");
        assert_eq!(r.rules[1].textures.len(), MAX_TEXTURES);
    }

    #[test]
    fn seeds_are_java_uuid_hash_codes() {
        // new UUID(0x0123456789abcdefL, 0xfedcba9876543210L).hashCode():
        // msb ^ lsb = 0xffffffffffffffff, high ^ low = 0.
        assert_eq!(
            seed(0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210_u64 as i64),
            0
        );
        // new UUID(1, 2).hashCode() = 3, and new UUID(1L << 32, 0) is 1.
        assert_eq!(seed(1, 2), 3);
        assert_eq!(seed(1 << 32, 0), 1);
    }
}
