//! Where a pack keeps its random entity textures: the numbered variants of each texture and the
//! [`Rules`] that choose between them.
//!
//! A texture `assets/<ns>/textures/<path>.png` has its variants in the first of these folders
//! that has any, each mirroring `textures/` (the `mob/` ones mirror `textures/entity/`):
//!
//! 1. ETF's `assets/<ns>/etf/random/<path>`
//! 2. `assets/<ns>/optifine/random/<path>`
//! 3. the legacy `assets/<ns>/optifine/mob/` and `assets/<ns>/mcpatcher/mob/`
//! 4. beside the texture itself under `textures/entity/`, ETF's extra: only when a
//!    `<name>.properties` sits there, so a pack without OptiFine files never changes a texture.
//!
//! Variant `N` of `creeper` is `creeper<N>.png`; of a name ending in a digit
//! (`warden_pulsating_spots_2`), `warden_pulsating_spots_2.<N>.png`. Variant 1 is the texture
//! itself unless a `<name>1.png` exists. `<name>.properties` holds the rules; a texture without
//! its own uses the one of the name it extends (`wolf_tame` and `wolf_angry` use
//! `wolf.properties`, `spider_eyes` uses `spider.properties`). With no rules at all, an entity
//! picks evenly between the texture and its variants 2, 3... up to the first missing number.
//!
//! Textures are named as the entity atlas names them: a vanilla texture by its path under
//! `textures/` (`entity/zombie/zombie`), another namespace's as `<ns>:<path>`, and a file
//! outside `textures/` by its namespace and its path under the namespace
//! (`minecraft:optifine/random/entity/zombie/zombie2`).
//!
//! Written from OptiFine's `random_entities.properties` and ETF's `RANDOM_GUIDE.md`; no mod's
//! source was read.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::paths::files_under;
use crate::properties::Properties;
use crate::random::{MAX_INDEX, Rules};

/// Every texture a pack varies, by its atlas name.
#[derive(Debug, Default)]
pub struct RandomTextures {
    entries: HashMap<String, Entry>,
    rules: Vec<Rules>,
}

/// One texture's variants.
#[derive(Debug, Default, PartialEq)]
pub struct Entry {
    /// Index into [`RandomTextures::rules`]: the rules that choose this texture's variant.
    pub rules: Option<usize>,
    /// Atlas names by variant index.
    variants: BTreeMap<u32, String>,
}

impl RandomTextures {
    pub fn scan(root: &Path) -> Self {
        let mut found: HashMap<String, [Source; SOURCES]> = HashMap::new();
        let Ok(namespaces) = fs::read_dir(root.join("assets")) else {
            return Self::default();
        };
        let mut namespaces: Vec<String> = namespaces
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        namespaces.sort();
        for ns in &namespaces {
            for (source, (folder, mirror)) in FOLDERS.iter().enumerate() {
                let folder = format!("assets/{ns}/{folder}");
                let files = files_under(root, &folder, &[".png", ".properties"]);
                for file in files {
                    let Some(inner) = file.strip_prefix(folder.as_str()) else {
                        continue;
                    };
                    add(&mut found, source, ns, &format!("{mirror}{inner}"), &file);
                }
            }
            // ETF: rules beside the entity texture, with variants there too.
            let textures = format!("assets/{ns}/textures/entity");
            for file in files_under(root, &textures, &[".properties"]) {
                let base = file.strip_suffix(".properties").unwrap_or(&file);
                let Some((dir, name)) = base.rsplit_once('/') else {
                    continue;
                };
                let inner = &file[format!("assets/{ns}/").len()..];
                add(&mut found, BESIDE, ns, inner, &file);
                for n in 1..=MAX_INDEX {
                    let png = format!("{dir}/{}.png", variant_stem(name, n));
                    if root.join(&png).is_file() {
                        let inner = &png[format!("assets/{ns}/").len()..];
                        add(&mut found, BESIDE, ns, inner, &png);
                    } else if n > 1 {
                        break;
                    }
                }
            }
        }
        let mut out = Self::default();
        let mut rules_by_file: HashMap<String, usize> = HashMap::new();
        for (texture, sources) in &found {
            let Some(source) = sources.iter().position(|s| match s.properties {
                Some(_) => true,
                None => !s.variants.is_empty(),
            }) else {
                continue;
            };
            let chosen = &sources[source];
            if source == BESIDE && chosen.properties.is_none() {
                continue;
            }
            // Its own rules, or those of the name it extends, in the same folder.
            let rules_file = chosen.properties.clone().or_else(|| {
                let mut name = texture.as_str();
                while let Some((shorter, _)) = name.rsplit_once('_') {
                    name = shorter;
                    if let Some(p) = found.get(name).and_then(|s| s[source].properties.clone()) {
                        return Some(p);
                    }
                }
                None
            });
            let rules = rules_file.and_then(|file| {
                if let Some(&id) = rules_by_file.get(&file) {
                    return Some(id);
                }
                let text = fs::read_to_string(root.join(&file)).ok()?;
                out.rules.push(Rules::parse(&Properties::parse(&text)));
                rules_by_file.insert(file, out.rules.len() - 1);
                Some(out.rules.len() - 1)
            });
            out.entries.insert(
                texture.clone(),
                Entry {
                    rules,
                    variants: chosen.variants.clone(),
                },
            );
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The variants of the texture with this atlas name.
    pub fn get(&self, texture: &str) -> Option<&Entry> {
        self.entries.get(texture)
    }

    pub fn rules(&self, id: usize) -> &Rules {
        &self.rules[id]
    }

    /// The atlas names of every variant texture, for the entity atlas to pack.
    pub fn textures(&self) -> impl Iterator<Item = &str> {
        self.entries
            .values()
            .flat_map(|e| e.variants.values().map(String::as_str))
    }
}

impl Entry {
    /// The atlas name of variant `index`, or `None` for the texture itself: variant 0, 1 without
    /// a `<name>1.png`, and a variant whose file is missing.
    pub fn texture(&self, index: u32) -> Option<&str> {
        self.variants.get(&index).map(String::as_str)
    }

    /// The even choice of an entity with this seed when no rules apply: the texture and its
    /// variants 2, 3... up to the first missing one.
    pub fn even(&self, seed: i32) -> u32 {
        let mut count = 1;
        while self.variants.contains_key(&(count + 1)) {
            count += 1;
        }
        seed.unsigned_abs() % count + 1
    }
}

/// The folders under `assets/<ns>/` that hold variants, in priority order, and the folder
/// under `assets/<ns>/` each mirrors. The entity atlas packs their images.
pub const FOLDERS: [(&str, &str); 4] = [
    ("etf/random", "textures"),
    ("optifine/random", "textures"),
    ("optifine/mob", "textures/entity"),
    ("mcpatcher/mob", "textures/entity"),
];

const SOURCES: usize = FOLDERS.len() + 1;
/// The ETF source beside the vanilla texture, last in priority.
const BESIDE: usize = 4;

#[derive(Clone, Debug, Default)]
struct Source {
    properties: Option<String>,
    variants: BTreeMap<u32, String>,
}

/// Records `file` (a path from the root) as `inner`, its mirrored path under `assets/<ns>/`
/// (`textures/entity/zombie/zombie2.png`): a texture's rules, or one of its variants.
fn add(
    found: &mut HashMap<String, [Source; SOURCES]>,
    source: usize,
    ns: &str,
    inner: &str,
    file: &str,
) {
    let Some((dir, name)) = inner.rsplit_once('/') else {
        return;
    };
    if let Some(stem) = name.strip_suffix(".properties") {
        let key = atlas_name(ns, &format!("{dir}/{stem}"));
        found.entry(key).or_default()[source].properties = Some(file.to_owned());
    } else if let Some((base, n)) = name.strip_suffix(".png").and_then(split_variant) {
        let key = atlas_name(ns, &format!("{dir}/{base}"));
        let file = file.strip_suffix(".png").unwrap_or(file);
        let variant = atlas_name(ns, &file[format!("assets/{ns}/").len()..]);
        found.entry(key).or_default()[source]
            .variants
            .insert(n, variant);
    }
}

/// `creeper7` → (`creeper`, 7); `spots_2.3` → (`spots_2`, 3). Not a variant: `creeper`,
/// `spots_2` (a name ending in a digit numbers its variants after a `.`), indices past
/// [`MAX_INDEX`].
fn split_variant(stem: &str) -> Option<(&str, u32)> {
    let digits = stem.len() - stem.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (base, n) = stem.split_at(stem.len() - digits);
    let n: u32 = n.parse().ok().filter(|&n| n <= MAX_INDEX)?;
    match base.strip_suffix('.') {
        Some(b) if b.ends_with(|c: char| c.is_ascii_digit()) => Some((b, n)),
        None if !base.is_empty() && !base.ends_with(|c: char| c.is_ascii_digit()) => {
            Some((base, n))
        }
        _ => None,
    }
}

fn variant_stem(name: &str, n: u32) -> String {
    if name.ends_with(|c: char| c.is_ascii_digit()) {
        format!("{name}.{n}")
    } else {
        format!("{name}{n}")
    }
}

/// A path under `assets/<ns>/` as the entity atlas names it.
fn atlas_name(ns: &str, inner: &str) -> String {
    match inner.strip_prefix("textures/") {
        Some(path) if ns == "minecraft" => path.to_owned(),
        Some(path) => format!("{ns}:{path}"),
        None => format!("{ns}:{inner}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Facts;

    fn pack(files: &[(&str, &str)]) -> (std::path::PathBuf, RandomTextures) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "horizon-random-textures-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        let found = RandomTextures::scan(&root);
        (root, found)
    }

    const RANDOM: &str = "assets/minecraft/optifine/random/entity";

    #[test]
    fn variants_and_rules_by_texture() {
        let (root, t) = pack(&[
            (&format!("{RANDOM}/creeper/creeper2.png"), ""),
            (&format!("{RANDOM}/creeper/creeper3.png"), ""),
            (&format!("{RANDOM}/creeper/creeper5.png"), ""),
            (
                &format!("{RANDOM}/warden/warden_pulsating_spots_2.2.png"),
                "",
            ),
            (
                &format!("{RANDOM}/warden/warden_pulsating_spots_22.png"),
                "",
            ),
            (
                &format!("{RANDOM}/wolf/wolf.properties"),
                "textures.1=2\nbaby.1=true",
            ),
            (&format!("{RANDOM}/wolf/wolf2.png"), ""),
            (&format!("{RANDOM}/wolf/wolf_tame2.png"), ""),
            ("assets/minecraft/textures/entity/creeper/creeper.png", ""),
        ]);
        let creeper = t.get("entity/creeper/creeper").unwrap();
        assert_eq!(creeper.rules, None);
        assert_eq!(
            creeper.texture(3),
            Some("minecraft:optifine/random/entity/creeper/creeper3")
        );
        assert_eq!(
            creeper.texture(1),
            None,
            "no creeper1.png: the texture itself"
        );
        assert_eq!(creeper.texture(4), None, "missing");
        // No rules: an even choice of 1, 2 and 3; 5 is past the gap at 4.
        assert_eq!(creeper.even(0), 1);
        assert_eq!(creeper.even(4), 2);
        assert_eq!(creeper.even(-5), 3);
        let spots = t.get("entity/warden/warden_pulsating_spots_2").unwrap();
        assert_eq!(
            spots.texture(2),
            Some("minecraft:optifine/random/entity/warden/warden_pulsating_spots_2.2")
        );
        assert_eq!(spots.texture(22), None, "spots_22 is no variant of spots_2");
        // wolf_tame has no rules of its own: it uses wolf.properties.
        let wolf = t.get("entity/wolf/wolf").unwrap();
        let tame = t.get("entity/wolf/wolf_tame").unwrap();
        assert!(wolf.rules.is_some());
        assert_eq!(tame.rules, wolf.rules);
        let baby = Facts {
            baby: Some(true),
            ..Default::default()
        };
        assert_eq!(t.rules(tame.rules.unwrap()).select(&baby, 0), 2);
        assert_eq!(t.textures().count(), 7);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folders_in_priority_order() {
        let (root, t) = pack(&[
            ("assets/minecraft/etf/random/entity/pig/pig2.png", ""),
            (&format!("{RANDOM}/pig/pig2.png"), ""),
            (&format!("{RANDOM}/pig/pig3.png"), ""),
            ("assets/minecraft/mcpatcher/mob/cow/cow2.png", ""),
            ("assets/minecraft/optifine/mob/sheep/sheep2.png", ""),
            ("assets/mymod/optifine/random/entity/bug/bug2.png", ""),
            // Beside the vanilla texture: only with a properties file.
            ("assets/minecraft/textures/entity/fox/fox2.png", ""),
            (
                "assets/minecraft/textures/entity/cat/cat.properties",
                "textures.1=2",
            ),
            ("assets/minecraft/textures/entity/cat/cat2.png", ""),
        ]);
        let pig = t.get("entity/pig/pig").unwrap();
        assert_eq!(pig.texture(2), Some("minecraft:etf/random/entity/pig/pig2"));
        assert_eq!(pig.texture(3), None, "only the ETF folder is used");
        assert_eq!(
            t.get("entity/cow/cow").unwrap().texture(2),
            Some("minecraft:mcpatcher/mob/cow/cow2")
        );
        assert_eq!(
            t.get("entity/sheep/sheep").unwrap().texture(2),
            Some("minecraft:optifine/mob/sheep/sheep2")
        );
        assert_eq!(
            t.get("mymod:entity/bug/bug").unwrap().texture(2),
            Some("mymod:optifine/random/entity/bug/bug2")
        );
        assert!(t.get("entity/fox/fox").is_none());
        let cat = t.get("entity/cat/cat").unwrap();
        assert_eq!(cat.texture(2), Some("entity/cat/cat2"));
        assert!(cat.rules.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn no_files_no_entries() {
        let (root, t) = pack(&[("assets/minecraft/textures/entity/pig/pig.png", "")]);
        assert!(t.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn variant_names() {
        assert_eq!(split_variant("creeper12"), Some(("creeper", 12)));
        assert_eq!(split_variant("spots_2.3"), Some(("spots_2", 3)));
        assert_eq!(split_variant("creeper"), None);
        // Variant 2 of `spots_`: a name ending in a digit is only ever a base texture.
        assert_eq!(split_variant("spots_2"), Some(("spots_", 2)));
        assert_eq!(split_variant("2"), None);
        assert_eq!(split_variant("a.3"), None, "a `.` only follows a digit");
        assert_eq!(split_variant("x99999"), None);
        assert_eq!(variant_stem("spots_2", 4), "spots_2.4");
        assert_eq!(variant_stem("cat", 4), "cat4");
    }
}
