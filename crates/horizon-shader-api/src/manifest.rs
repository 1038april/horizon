//! `horizon-shaders.toml`, a shader pack's manifest (schema 1, docs/shaders.md).
use serde::Deserialize;

/// The manifest's file name at the root of a pack.
pub const MANIFEST: &str = "horizon-shaders.toml";
/// The manifest schema this build reads.
pub const SCHEMA: u32 = 1;
/// The shader API line this build serves: the stages, their signatures and
/// the inputs a pack's WGSL may read.
pub const API: &str = "0.1";
/// Full-screen passes a pack may chain.
pub const MAX_POST_PASSES: usize = 8;
/// Options a pack may declare.
pub const MAX_OPTIONS: usize = 64;
/// Settings pages a pack may declare, and the items one page may hold.
pub const MAX_PAGES: usize = 32;
pub const MAX_PAGE_ITEMS: usize = 64;
/// The most GPU time a pack may declare for itself, in milliseconds.
pub const MAX_GPU_MS: f32 = 16.0;
/// `[[materials]]` entries a manifest may declare.
pub const MAX_MATERIALS: usize = 1024;
/// Sprite tags a pack may declare: the bits of `MaterialInput.tags`.
pub const MAX_TAGS: usize = 32;

/// A checked manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    /// `[a-z0-9_]{1,64}`: the pack's settings key and registry id.
    pub id: String,
    pub name: String,
    pub version: String,
    /// The API line the pack is written for.
    pub api: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    /// WGSL files put before every stage's own file, in order.
    pub shared: Vec<String>,
    pub sky: Option<Stage>,
    pub material: Option<Stage>,
    /// Full-screen passes, in the order they run.
    pub post: Vec<Stage>,
    pub options: Vec<PackOption>,
    /// How the settings page lays the options out; the first page is the one it opens on.
    /// Empty: one page with every option.
    pub pages: Vec<OptionPage>,
    /// Sprite tags the material stage reads, in bit order.
    pub tags: Vec<SpriteTag>,
    /// Material ids by sprite: the first entry with a matching pattern wins.
    pub materials: Vec<Material>,
    pub budget: Budget,
    /// When the material stage's water mirror (`reflection`) is drawn.
    pub reflection: Option<Reflection>,
    /// The shadow map the material and post stages read, if the pack asks for one.
    pub shadow: Option<Shadow>,
}

/// `[shadow]`: the renderer draws the solid terrain and entities as the sun (the moon by night)
/// sees them into a square depth map each frame, which the material and post stages read.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    /// The map's side, in texels.
    #[serde(default = "default_shadow_resolution")]
    pub resolution: u32,
    /// Half the side of the square the map covers around the camera, in blocks.
    #[serde(default = "default_shadow_distance")]
    pub distance: f32,
    /// `[[materials]]` ids drawn into the map whole, their cutouts ignored: leaves cast
    /// a full shadow, say, whatever holes they show.
    #[serde(default)]
    pub solid_materials: Vec<u32>,
}

/// The shadow map's side, in texels.
pub const SHADOW_RESOLUTIONS: std::ops::RangeInclusive<u32> = 256..=8192;
/// Half the side of the square the shadow map covers, in blocks.
pub const SHADOW_DISTANCES: std::ops::RangeInclusive<f32> = 32.0..=512.0;

fn default_shadow_resolution() -> u32 {
    2048
}

fn default_shadow_distance() -> f32 {
    128.0
}

/// `[reflection]`: when Horizon draws the water mirror the material stage reads as
/// `reflection`. Without it, whenever the stage reads that texture.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reflection {
    /// The option the mirror is drawn while it is not 0 (a bool on, a number other than 0),
    /// if any.
    #[serde(default)]
    pub option: Option<String>,
}

/// A set of block sprites the material stage can tell apart: its bit in
/// `MaterialInput.tags` is its index in the manifest.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpriteTag {
    /// A WGSL identifier, for the pack's own reading and its errors.
    pub id: String,
    /// Sprite names such as `minecraft:block/glass` (the namespace defaults to
    /// `minecraft`), each with at most one `*` matching any run of characters.
    pub sprites: Vec<String>,
}

impl SpriteTag {
    /// Whether the sprite `name` (`namespace:path`) is one of the tag's.
    pub fn matches(&self, name: &str) -> bool {
        self.sprites.iter().any(|p| sprite_matches(p, name))
    }
}

/// `[[materials]]`: a number the material stage reads as `MaterialInput.material`, for the
/// many kinds of surface a pack treats differently, where `[[tags]]` hold 32 sets.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// 1 to 65535 (0 is a sprite without an entry). Entries may share one.
    pub id: u32,
    /// Sprite names such as `minecraft:block/oak_leaves` (the namespace defaults to
    /// `minecraft`), each with any number of `*` matching any run of characters.
    pub sprites: Vec<String>,
}

impl Material {
    /// Whether the sprite `name` (`namespace:path`) has this entry's id.
    pub fn matches(&self, name: &str) -> bool {
        self.sprites.iter().any(|p| sprite_matches(p, name))
    }
}

/// Whether the sprite `name` (`namespace:path`) fits `pattern`.
fn sprite_matches(pattern: &str, name: &str) -> bool {
    let pattern = match pattern.contains(':') {
        true => std::borrow::Cow::Borrowed(pattern),
        false => std::borrow::Cow::Owned(format!("minecraft:{pattern}")),
    };
    let Some((head, rest)) = pattern.split_once('*') else {
        return pattern == name;
    };
    let Some(mut left) = name.strip_prefix(head) else {
        return false;
    };
    let (middle, tail) = rest.rsplit_once('*').unwrap_or(("", rest));
    for part in middle.split('*').filter(|p| !p.is_empty()) {
        let Some(at) = left.find(part) else {
            return false;
        };
        left = &left[at + part.len()..];
    }
    left.ends_with(tail)
}

/// One stage: the WGSL file that defines its function.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub file: String,
    /// WGSL files put before `file`, after the shared ones, for this stage only.
    #[serde(default)]
    pub files: Vec<String>,
    /// Shown in the GPU time overlay; the file name when absent.
    #[serde(default)]
    pub name: Option<String>,
    /// The material stage only: whether Horizon applies its fog after the pack's colour
    /// (true when absent); false for a pack that fogs its surfaces itself.
    #[serde(default)]
    pub fog: Option<bool>,
    /// The sky stage only: whether Minecraft's clouds still draw (true when absent); false for
    /// a pack whose sky draws its own.
    #[serde(default)]
    pub clouds: Option<bool>,
    /// The sky stage only: whether Minecraft's sun still draws over the sky (true when absent);
    /// false for a pack whose sky draws its own. The moon stays.
    #[serde(default)]
    pub sun: Option<bool>,
    /// The sky stage only: whether Minecraft's stars still draw over the sky (true when absent);
    /// false for a pack whose sky draws its own.
    #[serde(default)]
    pub stars: Option<bool>,
}

/// An option the player can change: a WGSL `override` constant of the
/// same name, set when the pipelines are built.
#[derive(Clone, Debug, PartialEq)]
pub struct PackOption {
    /// The `override`'s name in the pack's WGSL.
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: OptionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionKind {
    /// `override <id>: bool`.
    Bool { default: bool },
    /// `override <id>: f32`; the slider moves by `step`.
    Float {
        default: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    /// `override <id>: i32`.
    Int { default: i32, min: i32, max: i32 },
    /// `override <id>: u32`, the index of the chosen value.
    Choice { values: Vec<String>, default: u32 },
}

/// `[[pages]]`: a page of the pack's settings screen.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionPage {
    /// What `page:<id>` items link to.
    pub id: String,
    /// The page's title, and the label of the buttons linking to it.
    pub name: String,
    /// The tooltip of the buttons linking to it.
    pub description: String,
    /// 1 or 2: full-width buttons, or two a row.
    pub columns: u8,
    pub items: Vec<PageItem>,
}

/// What a page shows in one slot, in order: left to right, then down.
#[derive(Clone, Debug, PartialEq)]
pub enum PageItem {
    /// `"<id>"`: the option of that id.
    Option(usize),
    /// `"page:<id>"`: a button opening `Manifest::pages[i]`.
    Page(usize),
    /// `""`: an empty slot.
    Empty,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Budget {
    /// The GPU time the pack aims to add to a frame, in milliseconds; the
    /// overlay shows the pack over budget past it.
    pub gpu_ms: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema: u32,
    id: String,
    name: String,
    version: String,
    api: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    authors: Vec<String>,
    license: Option<String>,
    #[serde(default)]
    shared: Vec<String>,
    sky: Option<Stage>,
    material: Option<Stage>,
    #[serde(default)]
    post: Vec<Stage>,
    #[serde(default)]
    options: Vec<RawOption>,
    #[serde(default)]
    pages: Vec<RawPage>,
    #[serde(default)]
    tags: Vec<SpriteTag>,
    #[serde(default)]
    materials: Vec<Material>,
    #[serde(default)]
    budget: RawBudget,
    reflection: Option<Reflection>,
    shadow: Option<Shadow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOption {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "type")]
    kind: String,
    default: toml::Value,
    min: Option<toml::Value>,
    max: Option<toml::Value>,
    step: Option<f64>,
    values: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBudget {
    gpu_ms: Option<f64>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let raw: Raw = toml::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
        let err = |e: String| format!("{MANIFEST}: {e}");
        if raw.schema != SCHEMA {
            return Err(err(format!(
                "schema {}; this Horizon reads {SCHEMA}",
                raw.schema
            )));
        }
        if raw.api != API {
            return Err(err(format!(
                "written for shader API {}; this Horizon serves {API}",
                raw.api
            )));
        }
        let id_ok = (1..=64).contains(&raw.id.len())
            && raw
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            return Err(err(format!("id {:?} is not [a-z0-9_]{{1,64}}", raw.id)));
        }
        if raw.name.trim().is_empty() || raw.version.trim().is_empty() {
            return Err(err("name and version can't be empty".into()));
        }
        if raw.sky.is_none() && raw.material.is_none() && raw.post.is_empty() {
            return Err(err(
                "no stage: give it a [sky], a [material] or a [[post]]".into()
            ));
        }
        if raw.sky.iter().chain(&raw.post).any(|s| s.fog.is_some()) {
            return Err(err("`fog` is a key of [material] only".into()));
        }
        if raw.post.len() > MAX_POST_PASSES {
            return Err(err(format!(
                "{} post passes; a pack may chain {MAX_POST_PASSES}",
                raw.post.len()
            )));
        }
        let stages = raw.sky.iter().chain(&raw.material).chain(&raw.post);
        for file in raw
            .shared
            .iter()
            .chain(stages.flat_map(|s| s.files.iter().chain(std::iter::once(&s.file))))
        {
            if !crate::package::plain_path(file) || !file.ends_with(".wgsl") {
                return Err(err(format!("{file:?} is not a plain path to a .wgsl file")));
            }
        }
        let sky_only = |s: &Stage| s.clouds.is_some() || s.sun.is_some() || s.stars.is_some();
        if raw.material.iter().chain(&raw.post).any(sky_only) {
            return Err(err(
                "`clouds`, `sun` and `stars` are keys of [sky] only".into()
            ));
        }
        if raw.options.len() > MAX_OPTIONS {
            return Err(err(format!(
                "{} options; a pack may declare {MAX_OPTIONS}",
                raw.options.len()
            )));
        }
        let mut options = Vec::with_capacity(raw.options.len());
        for (i, o) in raw.options.into_iter().enumerate() {
            let id = o.id.clone();
            let option = o.check().map_err(|e| err(format!("option {id:?}: {e}")))?;
            if options[..i].iter().any(|p: &PackOption| p.id == option.id) {
                return Err(err(format!("option {id:?} is declared twice")));
            }
            options.push(option);
        }
        if !raw.tags.is_empty() && raw.material.is_none() {
            return Err(err(
                "tags are read by the material stage: give it a [material]".into(),
            ));
        }
        if raw.tags.len() > MAX_TAGS {
            return Err(err(format!(
                "{} tags; a pack may declare {MAX_TAGS}",
                raw.tags.len()
            )));
        }
        for (i, tag) in raw.tags.iter().enumerate() {
            let id = &tag.id;
            if !pack_identifier(id) {
                return Err(err(format!("tag {id:?}: the id must be a WGSL identifier")));
            }
            if raw.tags[..i].iter().any(|t| &t.id == id) {
                return Err(err(format!("tag {id:?} is declared twice")));
            }
            let sprite_ok = |p: &String| !p.is_empty() && p.matches('*').count() <= 1;
            if tag.sprites.is_empty() || !tag.sprites.iter().all(sprite_ok) {
                return Err(err(format!(
                    "tag {id:?}: needs sprites, each a name with at most one *"
                )));
            }
        }
        let solid_ids = raw.shadow.iter().any(|s| !s.solid_materials.is_empty());
        if !raw.materials.is_empty() && raw.material.is_none() && !solid_ids {
            return Err(err(
                "materials are read by the material stage and shadow.solid_materials: \
                 give it a [material] or use them there"
                    .into(),
            ));
        }
        if raw.materials.len() > MAX_MATERIALS {
            return Err(err(format!(
                "{} materials; a pack may declare {MAX_MATERIALS}",
                raw.materials.len()
            )));
        }
        for m in &raw.materials {
            if !(1..=65535).contains(&m.id) {
                return Err(err(format!("material id {} is not in 1..=65535", m.id)));
            }
            if m.sprites.is_empty() || m.sprites.iter().any(String::is_empty) {
                return Err(err(format!("material {}: needs sprites", m.id)));
            }
        }
        for id in raw.shadow.iter().flat_map(|s| &s.solid_materials) {
            if !raw.materials.iter().any(|m| m.id == *id) {
                return Err(err(format!(
                    "shadow.solid_materials: no [[materials]] entry has id {id}"
                )));
            }
        }
        let pages = check_pages(raw.pages, &options).map_err(err)?;
        let gpu_ms = match raw.budget.gpu_ms {
            Some(ms) if !(ms > 0.0 && ms <= f64::from(MAX_GPU_MS)) => {
                return Err(err(format!(
                    "budget.gpu_ms {ms} is not in (0, {MAX_GPU_MS}]"
                )));
            }
            ms => ms.map(|ms| ms as f32),
        };
        if let Some(r) = &raw.reflection {
            if raw.material.is_none() {
                return Err(err("[reflection] needs [material]".into()));
            }
            if let Some(id) = &r.option
                && !options.iter().any(|o| o.id == *id)
            {
                return Err(err(format!("reflection.option: `{id}` is no option")));
            }
        }
        if let Some(s) = &raw.shadow {
            if raw.material.is_none() && raw.post.is_empty() {
                return Err(err("[shadow] needs [material] or [[post]]".into()));
            }
            if !SHADOW_RESOLUTIONS.contains(&s.resolution) {
                return Err(err(format!(
                    "shadow.resolution {} is not in {SHADOW_RESOLUTIONS:?}",
                    s.resolution
                )));
            }
            if !SHADOW_DISTANCES.contains(&s.distance) {
                return Err(err(format!(
                    "shadow.distance {} is not in {SHADOW_DISTANCES:?}",
                    s.distance
                )));
            }
        }
        Ok(Manifest {
            id: raw.id,
            name: raw.name,
            version: raw.version,
            api: raw.api,
            description: raw.description,
            authors: raw.authors,
            license: raw.license,
            shared: raw.shared,
            sky: raw.sky,
            material: raw.material,
            post: raw.post,
            options,
            pages,
            tags: raw.tags,
            materials: raw.materials,
            budget: Budget { gpu_ms },
            reflection: raw.reflection,
            shadow: raw.shadow,
        })
    }

    /// The tags of each block sprite `(name, layer)` that has any, as `(layer, bits)` in
    /// layer order: what the material stage's `MaterialInput.tags` reads.
    pub fn sprite_tags<'a>(
        &self,
        sprites: impl IntoIterator<Item = (&'a str, u32)>,
    ) -> Vec<(u32, u32)> {
        // Identical stills share one layer under several names: their tags are or-ed
        // together, so each layer is one case of the shader's `switch`.
        let mut layers = std::collections::BTreeMap::<u32, u32>::new();
        for (name, layer) in sprites {
            let bits = self
                .tags
                .iter()
                .enumerate()
                .filter(|(_, t)| t.matches(name))
                .fold(0, |bits, (i, _)| bits | 1 << i);
            if bits != 0 {
                *layers.entry(layer).or_default() |= bits;
            }
        }
        layers.into_iter().collect()
    }

    /// The material id of each block sprite `(name, layer)` that has one, as `(layer, id)` in
    /// layer order: what the material stage's `MaterialInput.material` reads.
    pub fn sprite_materials<'a>(
        &self,
        sprites: impl IntoIterator<Item = (&'a str, u32)>,
    ) -> Vec<(u32, u32)> {
        // Identical stills share one layer under several names: the earliest entry any of
        // them matches decides, whatever order the names come in.
        let mut layers = std::collections::BTreeMap::<u32, usize>::new();
        for (name, layer) in sprites {
            if let Some(entry) = self.materials.iter().position(|m| m.matches(name)) {
                let best = layers.entry(layer).or_insert(entry);
                *best = entry.min(*best);
            }
        }
        layers
            .into_iter()
            .map(|(layer, entry)| (layer, self.materials[entry].id))
            .collect()
    }

    /// Every WGSL file the manifest names, shared files first, each once.
    pub fn files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = Vec::new();
        let stages = self.sky.iter().chain(&self.material).chain(&self.post);
        for file in self
            .shared
            .iter()
            .chain(stages.flat_map(|s| s.files.iter().chain(std::iter::once(&s.file))))
        {
            if !files.contains(&file.as_str()) {
                files.push(file);
            }
        }
        files
    }
}

/// A WGSL identifier the renderer leaves to packs: ASCII, not starting
/// with a digit, `_` or the renderer's own `horizon_` prefix.
pub fn pack_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && name.len() <= 64
        && !name.starts_with("horizon_")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPage {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_columns")]
    columns: u8,
    items: Vec<String>,
}

fn default_columns() -> u8 {
    2
}

/// `[[pages]]` checked against the options: known ids, each page's own id once, links to
/// other pages only, and every page and option reached from the first page.
fn check_pages(raw: Vec<RawPage>, options: &[PackOption]) -> Result<Vec<OptionPage>, String> {
    let pages = read_pages(&raw, options)?;
    if pages.is_empty() {
        return Ok(pages);
    }
    let (mut seen, mut next, mut shown) = (
        vec![false; pages.len()],
        vec![0],
        vec![false; options.len()],
    );
    seen[0] = true;
    while let Some(at) = next.pop() {
        for item in &pages[at].items {
            match *item {
                PageItem::Option(i) => shown[i] = true,
                PageItem::Page(i) if !seen[i] => {
                    seen[i] = true;
                    next.push(i);
                }
                _ => {}
            }
        }
    }
    if let Some(i) = seen.iter().position(|s| !s) {
        return Err(format!(
            "page {:?} can't be reached from the first page, {:?}",
            pages[i].id, pages[0].id
        ));
    }
    if let Some(i) = shown.iter().position(|s| !s) {
        return Err(format!(
            "option {:?} is on no page the settings reach",
            options[i].id
        ));
    }
    Ok(pages)
}

fn read_pages(raw: &[RawPage], options: &[PackOption]) -> Result<Vec<OptionPage>, String> {
    if raw.len() > MAX_PAGES {
        return Err(format!(
            "{} pages; a pack may declare {MAX_PAGES}",
            raw.len()
        ));
    }
    for (i, page) in raw.iter().enumerate() {
        if !pack_identifier(&page.id) {
            return Err(format!(
                "page {:?}: the id must be a letter, then letters, digits or _, up to 64",
                page.id
            ));
        }
        if raw[..i].iter().any(|p| p.id == page.id) {
            return Err(format!("page {:?} is declared twice", page.id));
        }
    }
    raw.iter()
        .enumerate()
        .map(|(at, page)| {
            let fail = |e: String| format!("page {:?}: {e}", page.id);
            if page.name.trim().is_empty() {
                return Err(fail("needs a name".into()));
            }
            if !(1..=2).contains(&page.columns) {
                return Err(fail(format!("columns = {}; 1 or 2", page.columns)));
            }
            if page.items.len() > MAX_PAGE_ITEMS {
                return Err(fail(format!(
                    "{} items; a page may hold {MAX_PAGE_ITEMS}",
                    page.items.len()
                )));
            }
            let mut items = Vec::with_capacity(page.items.len());
            for item in &page.items {
                let item = if item.is_empty() {
                    PageItem::Empty
                } else if let Some(link) = item.strip_prefix("page:") {
                    match raw.iter().position(|p| p.id == link) {
                        Some(i) if i == at => return Err(fail("links to itself".into())),
                        Some(i) => PageItem::Page(i),
                        None => return Err(fail(format!("no page {link:?} to link to"))),
                    }
                } else {
                    match options.iter().position(|o| o.id == *item) {
                        Some(i) => PageItem::Option(i),
                        None => return Err(fail(format!("no option {item:?}"))),
                    }
                };
                if item != PageItem::Empty && items.contains(&item) {
                    return Err(fail(format!(
                        "{:?} is on the page twice",
                        page.items[items.len()]
                    )));
                }
                items.push(item);
            }
            Ok(OptionPage {
                id: page.id.clone(),
                name: page.name.clone(),
                description: page.description.clone(),
                columns: page.columns,
                items,
            })
        })
        .collect()
}

impl RawOption {
    fn check(self) -> Result<PackOption, String> {
        if !pack_identifier(&self.id) {
            return Err(
                "the id must be a WGSL identifier: a letter, then letters, digits or _, up to 64, not starting with horizon_".into(),
            );
        }
        let int = |v: &toml::Value| {
            v.as_integer()
                .and_then(|i| i32::try_from(i).ok())
                .ok_or_else(|| format!("{v} is not a 32-bit integer"))
        };
        let float = |v: &toml::Value| {
            v.as_float()
                .or_else(|| v.as_integer().map(|i| i as f64))
                .filter(|f| f.is_finite())
                .map(|f| f as f32)
                .ok_or_else(|| format!("{v} is not a number"))
        };
        let need = |v: Option<toml::Value>, key: &str| v.ok_or_else(|| format!("needs {key}"));
        let unused = |present: bool, key: &str| {
            if present {
                Err(format!("{key} doesn't apply to type = {:?}", self.kind))
            } else {
                Ok(())
            }
        };
        let kind = match self.kind.as_str() {
            "bool" => {
                unused(self.min.is_some() || self.max.is_some(), "min/max")?;
                unused(self.step.is_some(), "step")?;
                unused(self.values.is_some(), "values")?;
                let default = self
                    .default
                    .as_bool()
                    .ok_or("default must be true or false")?;
                OptionKind::Bool { default }
            }
            "float" => {
                unused(self.values.is_some(), "values")?;
                let (min, max) = (
                    float(&need(self.min, "min")?)?,
                    float(&need(self.max, "max")?)?,
                );
                let default = float(&self.default)?;
                let step = self.step.map_or((max - min) / 100.0, |s| s as f32);
                if !(min < max && (min..=max).contains(&default)) {
                    return Err(format!("needs min < max and default in [{min}, {max}]"));
                }
                if !(step > 0.0 && step <= max - min) {
                    return Err(format!("step {step} is not in (0, max - min]"));
                }
                OptionKind::Float {
                    default,
                    min,
                    max,
                    step,
                }
            }
            "int" => {
                unused(self.step.is_some(), "step")?;
                unused(self.values.is_some(), "values")?;
                let (min, max) = (int(&need(self.min, "min")?)?, int(&need(self.max, "max")?)?);
                let default = int(&self.default)?;
                if !(min < max && (min..=max).contains(&default)) {
                    return Err(format!("needs min < max and default in [{min}, {max}]"));
                }
                OptionKind::Int { default, min, max }
            }
            "choice" => {
                unused(self.min.is_some() || self.max.is_some(), "min/max")?;
                unused(self.step.is_some(), "step")?;
                let values = self.values.ok_or("needs values")?;
                if !(2..=32).contains(&values.len()) {
                    return Err("needs 2 to 32 values".into());
                }
                if values
                    .iter()
                    .enumerate()
                    .any(|(i, v)| v.trim().is_empty() || values[..i].contains(v))
                {
                    return Err("values must be distinct and not empty".into());
                }
                let default = self
                    .default
                    .as_str()
                    .and_then(|d| values.iter().position(|v| v == d))
                    .ok_or("default must be one of the values")?;
                OptionKind::Choice {
                    values,
                    default: default as u32,
                }
            }
            other => {
                return Err(format!("type {other:?} is not bool, float, int or choice"));
            }
        };
        Ok(PackOption {
            id: self.id,
            name: self.name,
            description: self.description,
            kind,
        })
    }
}

impl OptionKind {
    /// The WGSL type of the option's `override`.
    pub fn wgsl_type(&self) -> &'static str {
        match self {
            OptionKind::Bool { .. } => "bool",
            OptionKind::Float { .. } => "f32",
            OptionKind::Int { .. } => "i32",
            OptionKind::Choice { .. } => "u32",
        }
    }

    /// The default as the pipeline constant wgpu takes.
    pub fn default_constant(&self) -> f64 {
        match *self {
            OptionKind::Bool { default } => f64::from(u8::from(default)),
            OptionKind::Float { default, .. } => f64::from(default),
            OptionKind::Int { default, .. } => f64::from(default),
            OptionKind::Choice { default, .. } => f64::from(default),
        }
    }
}
