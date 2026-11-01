//! Custom Item Textures: the rules of `optifine/cit/**/*.properties` and the pack's global
//! `cit.properties`, parsed into what they test on a stack and what they change. Matching stacks
//! against them is the client's (`horizon_client::cit`): it knows the items and their components.
//!
//! Written from OptiFine's `cit.properties` and `cit_single.properties` docs, CIT Resewn's wiki
//! (citresewn.shcm.io) and CIT Resewn Continuation's MIT-licensed source (its conditions, read
//! for the 1.21 component rules); where OptiFine and CIT Resewn differ, CIT Resewn is followed.

use crate::matchers::{IdList, Name, RangeList, namespaced};
use crate::paths::{resolve, valid};
use crate::properties::Properties;

/// What a rule changes (`type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Item,
    Armor,
    Elytra,
    Enchantment,
}

/// `hand`: the hand the item is drawn in; GUI items count as the main hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Any,
    Main,
    Off,
}

/// `damage` (with `%` against the item's max damage) and `damageMask`.
#[derive(Clone, Debug, PartialEq)]
pub struct Damage {
    pub ranges: RangeList,
    pub percent: bool,
    pub mask: Option<i32>,
}

impl Damage {
    pub fn matches(&self, damage: u32, max_damage: u32) -> bool {
        let mut value = damage as i32;
        if let Some(mask) = self.mask {
            value &= mask;
        }
        if !self.percent {
            return self.ranges.contains(value);
        }
        if max_damage == 0 {
            return false;
        }
        let percent = 100.0 * f64::from(value) / f64::from(max_damage);
        self.ranges
            .0
            .iter()
            .any(|&(lo, hi)| f64::from(lo) <= percent && percent <= f64::from(hi))
    }
}

/// Where a component test reads its value.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// A data component by id (`minecraft:custom_name`), then a path inside its NBT.
    Component(String),
    /// Pre-1.20.5 `Enchantments` / `StoredEnchantments`: a list of `{id, lvl}` compounds, which
    /// the client builds from the stack's enchantments.
    LegacyEnchantments { stored: bool },
}

/// One `components.<id>.<path>=<value>` or `nbt.<path>=<value>` test.
#[derive(Clone, Debug)]
pub struct ComponentTest {
    pub source: Source,
    /// Compound keys and list indexes; `*` is any key or element, `count` a list's length. The
    /// tag walked decides which a segment is: `0` is an index in a list, a key in a compound.
    pub path: Vec<String>,
    pub value: ValueTest,
    /// A leading `!`: the test passes when the value does not match.
    pub negate: bool,
}

/// The right-hand side of a component test.
#[derive(Clone, Debug)]
pub enum ValueTest {
    /// `exists:true` / `exists:false`.
    Exists(bool),
    /// `range:1 5 7-9`: an integer value.
    Range(RangeList),
    /// `raw:<name>`: the value written as SNBT.
    Raw(Name),
    /// `pattern:`, `regex:` (and their `i` forms) against a string, a text component's plain
    /// text, or a number's decimal form.
    Text(Name),
    /// A bare value: the same string, the same number (`#rrggbb` and `0x` hex are integers), or
    /// for a compound or list, SNBT that the value contains.
    Literal(Literal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Literal {
    pub text: String,
    pub int: Option<i64>,
    pub float: Option<f64>,
}

impl Literal {
    fn parse(text: &str) -> Self {
        let hex = text
            .strip_prefix('#')
            .or_else(|| text.strip_prefix("0x"))
            .and_then(|h| i64::from_str_radix(h, 16).ok());
        Self {
            text: text.to_owned(),
            int: hex.or_else(|| text.parse().ok()),
            float: text.parse().ok(),
        }
    }
}

impl ValueTest {
    fn parse(value: &str) -> Option<(Self, bool)> {
        let (negate, value) = match value.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, value),
        };
        let test = if let Some(v) = value.strip_prefix("exists:") {
            ValueTest::Exists(v.trim().parse().ok()?)
        } else if let Some(v) = value.strip_prefix("range:") {
            ValueTest::Range(RangeList::parse(v)?)
        } else if let Some(v) = value.strip_prefix("raw:") {
            ValueTest::Raw(Name::parse(v)?)
        } else if ["pattern:", "ipattern:", "regex:", "iregex:"]
            .iter()
            .any(|p| value.starts_with(p))
        {
            ValueTest::Text(Name::parse(value)?)
        } else {
            ValueTest::Literal(Literal::parse(value))
        };
        Some((test, negate))
    }
}

impl ComponentTest {
    /// `key` is the part after `nbt.` or `components.`.
    fn parse(legacy: bool, key: &str, value: &str) -> Option<Self> {
        let key = key.replace('~', "minecraft:");
        let mut segments: Vec<String> = key.split('.').map(str::to_owned).collect();
        if segments.iter().any(String::is_empty) {
            return None;
        }
        let (source, mut path) = if legacy {
            legacy_source(segments)
        } else {
            let id = namespaced(&segments.remove(0));
            (Source::Component(id), segments)
        };
        // Enchantments were `{levels: {id: level}}` until 1.21.5 and are the bare map since, as
        // OptiFine's doc still writes them (`components.enchantments.levels.~thorns`).
        if let Source::Component(id) = &source {
            if matches!(
                &**id,
                "minecraft:enchantments" | "minecraft:stored_enchantments"
            ) && path.first().is_some_and(|s| s == "levels")
            {
                path.remove(0);
            }
        }
        let (value, negate) = ValueTest::parse(value)?;
        Some(Self {
            source,
            path,
            value,
            negate,
        })
    }
}

/// Where a pre-1.20.5 `nbt.` path lives since items moved to data components. Tags that became
/// no component of their own were moved into `minecraft:custom_data` by the game's data fixer,
/// so every other path is read there.
fn legacy_source(path: Vec<String>) -> (Source, Vec<String>) {
    let component = |id: &str, prefix: &[&str], rest: &[String]| {
        let path = prefix.iter().map(|s| (*s).to_owned());
        (
            Source::Component(format!("minecraft:{id}")),
            path.chain(rest.iter().cloned()).collect(),
        )
    };
    let strs: Vec<&str> = path.iter().map(String::as_str).collect();
    match strs[..] {
        ["display", "Name", ..] => component("custom_name", &[], &path[2..]),
        ["display", "Lore", ..] => component("lore", &[], &path[2..]),
        ["display", "color", ..] => component("dyed_color", &[], &path[2..]),
        ["CustomModelData", ..] => component("custom_model_data", &["floats", "0"], &path[1..]),
        ["Potion", ..] => component("potion_contents", &["potion"], &path[1..]),
        ["CustomPotionColor", ..] => component("potion_contents", &["custom_color"], &path[1..]),
        ["Damage", ..] => component("damage", &[], &path[1..]),
        ["RepairCost", ..] => component("repair_cost", &[], &path[1..]),
        ["Trim", ..] => component("trim", &[], &path[1..]),
        ["title", ..] => component("written_book_content", &["title", "raw"], &path[1..]),
        ["author", ..] => component("written_book_content", &["author"], &path[1..]),
        ["Enchantments", ..] => (
            Source::LegacyEnchantments { stored: false },
            path[1..].to_vec(),
        ),
        ["StoredEnchantments", ..] => (
            Source::LegacyEnchantments { stored: true },
            path[1..].to_vec(),
        ),
        _ => component("custom_data", &[], &path),
    }
}

/// One `.properties` file under `optifine/cit/`.
#[derive(Clone, Debug)]
pub struct Rule {
    /// The file's resource path: the tie-breaker between rules of equal weight.
    pub path: String,
    pub kind: Kind,
    pub weight: i32,
    /// Namespaced item ids; empty (enchantment rules only) is any item.
    pub items: Vec<String>,
    pub stack_size: Option<RangeList>,
    pub damage: Option<Damage>,
    pub hand: Hand,
    /// Namespaced enchantment ids, any of which the stack must have.
    pub enchantments: Vec<String>,
    /// Levels one of the stack's enchantments (of `enchantments`, when given) must have.
    pub enchantment_levels: Option<RangeList>,
    pub components: Vec<ComponentTest>,
    /// `texture`, as a resource path to a `.png`.
    pub texture: Option<String>,
    /// `texture.<name>`: (name, resource path).
    pub textures: Vec<(String, String)>,
    /// `model`, as a resource path to a `.json`.
    pub model: Option<String>,
    /// `model.<name>`.
    pub models: Vec<(String, String)>,
}

impl Rule {
    /// The rule `props` (at resource path `path`) describes. `exists` answers whether a resource
    /// path names a file, to find textures and models the way OptiFine and CIT Resewn look for
    /// them. `None` for an unknown type, a malformed condition, or a rule that changes nothing.
    pub fn parse(path: &str, props: &Properties, exists: impl Fn(&str) -> bool) -> Option<Self> {
        let kind = match props.get("type").unwrap_or("item") {
            "item" => Kind::Item,
            "armor" => Kind::Armor,
            "elytra" => Kind::Elytra,
            "enchantment" => Kind::Enchantment,
            _ => return None,
        };
        let stem = path
            .rsplit('/')
            .next()?
            .strip_suffix(".properties")
            .unwrap_or_default();
        let mut items: Vec<String> = props
            .get("items")
            .or_else(|| props.get("matchItems"))
            .map(|v| IdList::parse(v).include)
            .unwrap_or_default();
        if kind == Kind::Elytra {
            items = vec!["minecraft:elytra".to_owned()];
        } else if items.is_empty() && kind != Kind::Enchantment {
            // CIT Resewn: a rule named after an item is for that item (a bare id, so a
            // `minecraft:` one whatever the rule's namespace).
            items.push(namespaced(stem));
        }
        // An optional key: `Some(None)` when absent, `None` (dropping the rule) when malformed.
        let optional = |key: &str, parse: &dyn Fn(&str) -> Option<i32>| match props.get(key) {
            Some(v) => parse(v).map(Some),
            None => Some(None),
        };
        let int = |v: &str| v.parse::<i32>().ok();
        let ranges = |key: &str| match props.get(key) {
            Some(v) => RangeList::parse(v).map(Some),
            None => Some(None),
        };
        let damage = match props.get("damage") {
            Some(v) => Some(Damage {
                ranges: RangeList::parse(&v.replace('%', ""))?,
                percent: v.contains('%'),
                mask: optional("damageMask", &int)?,
            }),
            None => None,
        };
        let enchantments = props
            .get("enchantments")
            .or_else(|| props.get("enchantmentIDs"))
            .map(|v| v.split_whitespace().map(enchantment_id).collect())
            .unwrap_or_default();
        let mut components = Vec::new();
        // `component.` is CIT Resewn's alias of `components.`.
        for (legacy, prefix) in [
            (true, "nbt."),
            (false, "components."),
            (false, "component."),
        ] {
            for (key, value) in props.with_prefix(prefix) {
                components.push(ComponentTest::parse(legacy, key, value)?);
            }
        }
        let tex = |v: &str| asset(path, v, "png", "textures", &exists);
        let model_at = |v: &str| asset(path, v, "json", "models", &exists);
        let mut texture = props.get("texture").and_then(tex);
        let mut model = props.get("model").and_then(model_at);
        let textures: Vec<_> = props
            .with_prefix("texture.")
            .filter_map(|(name, v)| Some((name.to_owned(), tex(v)?)))
            .collect();
        let models: Vec<_> = props
            .with_prefix("model.")
            .filter_map(|(name, v)| Some((name.to_owned(), model_at(v)?)))
            .collect();
        let named = ["texture", "model"].iter().any(|k| props.get(k).is_some())
            || props.with_prefix("texture.").next().is_some()
            || props.with_prefix("model.").next().is_some();
        if !named {
            // Neither named: a model, then a texture, named after the file beside it.
            let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
            let beside = |ext: &str| {
                let p = format!("{dir}/{stem}.{ext}");
                exists(&p).then_some(p)
            };
            if kind == Kind::Item {
                model = beside("json");
            }
            if model.is_none() {
                texture = beside("png");
            }
        }
        if texture.is_none() && model.is_none() && textures.is_empty() && models.is_empty() {
            return None;
        }
        Some(Self {
            path: path.to_owned(),
            kind,
            weight: optional("weight", &int)?.unwrap_or(0),
            items,
            stack_size: ranges("stackSize")?,
            damage,
            hand: match props.get("hand") {
                None | Some("any") => Hand::Any,
                Some("main") => Hand::Main,
                Some("off") => Hand::Off,
                Some(_) => return None,
            },
            enchantments,
            enchantment_levels: ranges("enchantmentLevels")?,
            components,
            texture,
            textures,
            model,
            models,
        })
    }

    /// Which of two matching rules is chosen: `Less` for the one that wins, the higher weight,
    /// then the path that sorts first.
    pub fn priority(&self, other: &Self) -> std::cmp::Ordering {
        (other.weight, &self.path).cmp(&(self.weight, &other.path))
    }
}

/// A `texture`/`model` value as a resource path to an existing `.ext` file, trying what CIT
/// Resewn tries in its order: as written (OptiFine's paths, see `paths::resolve`, or a full
/// `assets/...` path), in the namespace's typed folder (`item/x` → `textures/item/x.png`), then
/// beside the rule (`sub/x`, and `./../x` up from its folder).
fn asset(
    props: &str,
    value: &str,
    ext: &str,
    typed: &str,
    exists: &impl Fn(&str) -> bool,
) -> Option<String> {
    let last = value.rsplit('/').next().unwrap_or(value);
    let value = if last.ends_with(&format!(".{ext}")) {
        value.to_owned()
    } else {
        format!("{value}.{ext}")
    };
    let dir = props.rsplit_once('/').map_or("", |(d, _)| d);
    let mut candidates = Vec::new();
    if value.split('/').any(|s| s == "..") {
        // Only up from the rule's folder: `~/` and `ns:` paths name their folder outright.
        if !value.starts_with("~/") && !value.contains(':') {
            candidates.extend(up_from(dir, &value));
        }
    } else if value.starts_with("assets/") {
        candidates.extend(valid(&value).then(|| value.clone()));
    } else if let Some(resolved) = resolve(props, &value) {
        let plain = !value.starts_with("~/") && !value.starts_with("./");
        if plain && value.contains(['/', ':']) {
            if let Some((ns, rest)) = resolved
                .strip_prefix("assets/")
                .and_then(|p| p.split_once('/'))
            {
                if !rest.starts_with(&format!("{typed}/")) {
                    candidates.push(format!("assets/{ns}/{typed}/{rest}"));
                }
            }
            if !value.contains(':') {
                candidates.push(format!("{dir}/{value}"));
            }
        }
        candidates.insert(0, resolved);
    }
    candidates.into_iter().find(|p| exists(p))
}

/// `value` (`./../x`, `../x`) in the folder `dir`, each `..` taking a folder off; `None` when it
/// would climb out of `assets/<namespace>/`.
fn up_from(dir: &str, value: &str) -> Option<String> {
    let mut out: Vec<&str> = dir.split('/').collect();
    for segment in value.strip_prefix("./").unwrap_or(value).split('/') {
        match segment {
            ".." if out.len() > 2 => {
                out.pop();
            }
            ".." => return None,
            s => out.push(s),
        }
    }
    let path = out.join("/");
    (out.len() > 2 && out[0] == "assets" && valid(&path)).then_some(path)
}

/// An `enchantments` entry: an id (`flame`, `minecraft:flame`) or a pre-1.13 numeric id. An
/// unknown number stays an id no enchantment has, as in CIT Resewn, so its rule never matches.
fn enchantment_id(token: &str) -> String {
    if token.eq_ignore_ascii_case("sweeping") || token.eq_ignore_ascii_case("minecraft:sweeping") {
        return "minecraft:sweeping_edge".to_owned();
    }
    let Ok(n) = token.parse::<u32>() else {
        return namespaced(token);
    };
    let name = match n {
        0 => "protection",
        1 => "fire_protection",
        2 => "feather_falling",
        3 => "blast_protection",
        4 => "projectile_protection",
        5 => "respiration",
        6 => "aqua_affinity",
        7 => "thorns",
        8 => "depth_strider",
        9 => "frost_walker",
        10 => "binding_curse",
        16 => "sharpness",
        17 => "smite",
        18 => "bane_of_arthropods",
        19 => "knockback",
        20 => "fire_aspect",
        21 => "looting",
        22 => "sweeping_edge",
        32 => "efficiency",
        33 => "silk_touch",
        34 => "unbreaking",
        35 => "fortune",
        48 => "power",
        49 => "punch",
        50 => "flame",
        51 => "infinity",
        61 => "luck_of_the_sea",
        62 => "lure",
        70 => "mending",
        71 => "vanishing_curse",
        _ => return namespaced(token),
    };
    format!("minecraft:{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(path: &str, text: &str, files: &[&str]) -> Option<Rule> {
        Rule::parse(path, &Properties::parse(text), |p| files.contains(&p))
    }

    const SWORD: &str = "assets/minecraft/optifine/cit/swords/flame.properties";

    #[test]
    fn an_item_rule_with_every_condition() {
        let r = rule(
            SWORD,
            "items=diamond_sword minecraft:iron_sword\ntexture=flame_sword\nweight=5\n\
             stackSize=1\ndamage=0-50%\nhand=off\nenchantmentIDs=20 sharpness\n\
             enchantmentLevels=3-\nnbt.display.Name=ipattern:*flame*\n",
            &["assets/minecraft/optifine/cit/swords/flame_sword.png"],
        )
        .unwrap();
        assert_eq!(r.kind, Kind::Item);
        assert_eq!(r.items, ["minecraft:diamond_sword", "minecraft:iron_sword"]);
        assert_eq!(
            r.texture.as_deref(),
            Some("assets/minecraft/optifine/cit/swords/flame_sword.png")
        );
        assert_eq!(r.weight, 5);
        assert_eq!(r.hand, Hand::Off);
        let stack = r.stack_size.as_ref().unwrap();
        assert!(stack.contains(1) && !stack.contains(2));
        assert_eq!(
            r.enchantments,
            ["minecraft:fire_aspect", "minecraft:sharpness"]
        );
        assert!(r.enchantment_levels.as_ref().unwrap().contains(7));
        assert!(!r.enchantment_levels.as_ref().unwrap().contains(2));
        // A diamond sword lasts 1561: half of it is 780.5 used.
        let damage = r.damage.as_ref().unwrap();
        assert!(damage.matches(780, 1561));
        assert!(!damage.matches(781, 1561));
        assert!(!damage.matches(0, 0), "no max damage, no percentage");
        let [name] = &r.components[..] else { panic!() };
        assert_eq!(
            name.source,
            Source::Component("minecraft:custom_name".into())
        );
        assert!(name.path.is_empty());
        let ValueTest::Text(n) = &name.value else {
            panic!()
        };
        assert!(n.matches("The FLAME Blade"));
    }

    #[test]
    fn damage_masks_apply_before_the_ranges() {
        let r = rule(
            SWORD,
            "items=potion\ndamage=3\ndamageMask=15\n",
            &["assets/minecraft/optifine/cit/swords/flame.png"],
        )
        .unwrap();
        let d = r.damage.unwrap();
        assert!(
            d.matches(16_387, 0),
            "splash fire resistance: 16387 & 15 = 3"
        );
        assert!(!d.matches(16_388, 0));
    }

    #[test]
    fn default_items_and_assets_come_from_the_file_name() {
        let dir = "assets/minecraft/optifine/cit/sticks";
        let path = format!("{dir}/stick.properties");
        // No texture or model named: the model beside it wins over the texture.
        let both = [&*format!("{dir}/stick.json"), &*format!("{dir}/stick.png")];
        let r = rule(&path, "nbt.CustomModelData=1\n", &both).unwrap();
        assert_eq!(r.items, ["minecraft:stick"]);
        assert_eq!(r.model.as_deref(), Some(&*format!("{dir}/stick.json")));
        assert_eq!(r.texture, None);
        let r = rule(&path, "", &both[1..]).unwrap();
        assert_eq!(r.texture.as_deref(), Some(&*format!("{dir}/stick.png")));
        // Nothing to draw: no rule.
        assert!(rule(&path, "", &[]).is_none());
    }

    #[test]
    fn asset_paths_try_the_typed_folders() {
        let files = [
            "assets/minecraft/textures/item/excalibur.png",
            "assets/minecraft/models/item/excalibur.json",
            "assets/minecraft/optifine/cit/shared/glow.png",
            "assets/minecraft/optifine/cit/swords/sub/x.png",
        ];
        let tex = |v| rule(SWORD, &format!("items=stick\ntexture={v}"), &files)?.texture;
        let at = |s: &str| Some(s.to_owned());
        assert_eq!(tex("item/excalibur"), at(files[0]));
        assert_eq!(tex("minecraft:item/excalibur.png"), at(files[0]));
        assert_eq!(tex("textures/item/excalibur"), at(files[0]));
        assert_eq!(
            tex("assets/minecraft/textures/item/excalibur.png"),
            at(files[0])
        );
        assert_eq!(tex("~/cit/shared/glow"), at(files[2]));
        assert_eq!(tex("./sub/x"), at(files[3]));
        assert_eq!(tex("./../swords/sub/x"), at(files[3]));
        assert_eq!(tex("sub/x"), at(files[3]), "beside the rule, last");
        assert_eq!(tex("~/cit/../cit/shared/glow"), None);
        let model = rule(SWORD, "items=stick\nmodel=item/excalibur", &files).unwrap();
        assert_eq!(model.model.as_deref(), Some(files[1]));
    }

    #[test]
    fn climbing_stops_at_the_namespace() {
        // Files at every folder a climb could reach: only the namespace's own are found.
        let files = ["assets/x.png", "x.png", "assets/minecraft/x.png"];
        let tex = |v: &str| rule(SWORD, &format!("items=stick\ntexture={v}"), &files)?.texture;
        assert_eq!(tex("../../../x").as_deref(), Some("assets/minecraft/x.png"));
        assert_eq!(tex("../../../../x"), None);
        assert_eq!(tex("../../../../../x"), None);
        assert_eq!(tex("..\\..\\..\\x"), None, "a Windows separator");
    }

    #[test]
    fn a_named_asset_that_is_missing_drops_the_rule() {
        // flame.png sits beside flame.properties, but the rule names another texture.
        let beside = ["assets/minecraft/optifine/cit/swords/flame.png"];
        assert!(rule(SWORD, "items=stick\ntexture=typo", &beside).is_none());
        assert!(rule(SWORD, "items=stick", &beside).is_some());
    }

    #[test]
    fn sub_textures_and_armor_layers() {
        let dir = "assets/minecraft/optifine/cit/bow";
        let files: Vec<String> = ["a0", "a1", "diamond1", "diamond2"]
            .iter()
            .map(|n| format!("{dir}/{n}.png"))
            .collect();
        let refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let r = rule(
            &format!("{dir}/bow.properties"),
            "items=bow\ntexture.bow_pulling_0=a0\ntexture.bow_pulling_1=a1\ntexture.bow_pulling_2=missing\n",
            &refs,
        )
        .unwrap();
        let names: Vec<_> = r.textures.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            ["bow_pulling_0", "bow_pulling_1"],
            "a missing file is dropped"
        );
        let armor = rule(
            &format!("{dir}/armor.properties"),
            "type=armor\nitems=diamond_chestplate\ntexture.diamond_layer_1=diamond1\ntexture.diamond_layer_2=diamond2\n",
            &refs,
        )
        .unwrap();
        assert_eq!(armor.kind, Kind::Armor);
        assert_eq!(
            armor.textures[1],
            ("diamond_layer_2".into(), files[3].clone())
        );
        let glint = rule(
            &format!("{dir}/glint.properties"),
            "type=enchantment\ntexture=a0",
            &refs,
        )
        .unwrap();
        assert!(
            glint.items.is_empty(),
            "an enchantment rule without items is for every item"
        );
        // Elytra rules are always for the elytra.
        let elytra = rule(
            &format!("{dir}/x.properties"),
            "type=elytra\nitems=stick\ntexture=a0",
            &refs,
        )
        .unwrap();
        assert_eq!(elytra.items, ["minecraft:elytra"]);
    }

    #[test]
    fn legacy_nbt_paths_land_on_components() {
        let source = |key: &str| {
            let t = ComponentTest::parse(true, key, "x").unwrap();
            let Source::Component(id) = t.source else {
                return (format!("{:?}", t.source), t.path);
            };
            (id, t.path)
        };
        let s = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            source("display.Lore.*"),
            ("minecraft:lore".into(), s(&["*"]))
        );
        assert_eq!(
            source("CustomModelData"),
            ("minecraft:custom_model_data".into(), s(&["floats", "0"]))
        );
        assert_eq!(
            source("Potion"),
            ("minecraft:potion_contents".into(), s(&["potion"]))
        );
        assert_eq!(
            source("ExtraAttributes.id"),
            (
                "minecraft:custom_data".into(),
                s(&["ExtraAttributes", "id"])
            )
        );
        let ench = ComponentTest::parse(true, "StoredEnchantments.0.id", "x").unwrap();
        assert_eq!(ench.source, Source::LegacyEnchantments { stored: true });
        assert_eq!(ench.path, s(&["0", "id"]));
        // `~` is the minecraft namespace; a bare component id is in it too.
        // Since 1.21.5 the enchantments component has no `levels` level.
        let c = ComponentTest::parse(false, "enchantments.levels.~thorns", "3").unwrap();
        assert_eq!(c.source, Source::Component("minecraft:enchantments".into()));
        assert_eq!(c.path, s(&["minecraft:thorns"]));
        let c = ComponentTest::parse(false, "custom_data.levels", "3").unwrap();
        assert_eq!(c.path, s(&["levels"]));
        assert!(ComponentTest::parse(false, "lore..0", "x").is_none());
    }

    #[test]
    fn component_values() {
        let v = |s: &str| ValueTest::parse(s).unwrap();
        let (ValueTest::Literal(l), false) = v("#ff99cc") else {
            panic!()
        };
        assert_eq!(l.int, Some(0xff99cc));
        let (ValueTest::Literal(l), false) = v("0x1F") else {
            panic!()
        };
        assert_eq!(l.int, Some(31));
        let (ValueTest::Literal(l), false) = v("1.5") else {
            panic!()
        };
        assert_eq!((l.int, l.float), (None, Some(1.5)));
        let (ValueTest::Literal(l), false) = v("53") else {
            panic!()
        };
        assert_eq!((l.int, l.float), (Some(53), Some(53.0)));
        let (ValueTest::Literal(l), true) = v("!Bob") else {
            panic!()
        };
        assert_eq!((l.text.as_str(), l.int), ("Bob", None));
        let (ValueTest::Range(r), false) = v("range:(-25) (-10)-10 15") else {
            panic!()
        };
        assert!(r.contains(-25) && r.contains(0) && !r.contains(11));
        assert!(matches!(
            v("exists:false"),
            (ValueTest::Exists(false), false)
        ));
        let (ValueTest::Raw(n), false) = v("raw:pattern:*Chopper*") else {
            panic!()
        };
        assert!(n.matches("{text:\"Chopper\"}"));
        assert!(ValueTest::parse("regex:(").is_none());
        assert!(ValueTest::parse("exists:maybe").is_none());
    }

    #[test]
    fn unknown_enchantment_ids_never_match_instead_of_matching_all() {
        let files = ["assets/minecraft/optifine/cit/swords/flame.png"];
        let r = rule(SWORD, "items=stick\nenchantmentIDs=99 sweeping", &files).unwrap();
        assert_eq!(r.enchantments, ["minecraft:99", "minecraft:sweeping_edge"]);
    }

    #[test]
    fn malformed_conditions_drop_the_rule() {
        let files = ["assets/minecraft/optifine/cit/swords/flame.png"];
        for bad in [
            "damage=lots",
            "damage=3\ndamageMask=abc",
            "stackSize=1-x",
            "hand=left",
            "weight=heavy",
            "type=hat",
            "nbt.display.Name=regex:(",
        ] {
            assert!(
                rule(SWORD, &format!("items=stick\n{bad}\n"), &files).is_none(),
                "{bad}"
            );
        }
    }

    #[test]
    fn heavier_rules_then_earlier_paths_win() {
        let files = ["assets/minecraft/optifine/cit/a.png"];
        let at = |path: &str, weight: i32| {
            rule(
                path,
                &format!("items=stick\ntexture=~/cit/a\nweight={weight}"),
                &files,
            )
            .unwrap()
        };
        let a = at("assets/minecraft/optifine/cit/a.properties", 0);
        let b = at("assets/minecraft/optifine/cit/b.properties", 0);
        let heavy = at("assets/minecraft/optifine/cit/c.properties", 3);
        let mut rules = [b, heavy, a];
        rules.sort_by(Rule::priority);
        let order: Vec<_> = rules
            .iter()
            .map(|r| r.path.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(order, ["c.properties", "a.properties", "b.properties"]);
    }
}
