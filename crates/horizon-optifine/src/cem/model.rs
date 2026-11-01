//! `.jem` entity models and the `.jpm` part models they load, baked into the geometry vanilla's
//! `ModelPart` draws: quads in model pixels on vanilla's axes (y down), UVs a fraction of the
//! texture.
//!
//! The files place parts the way Blockbench's OptiFine export writes them: `invertAxis` names the
//! axes that point the other way from vanilla's (`"xy"` in practice), a part's own
//! `translate` is minus its pivot and its boxes and submodels sit where the model has them (the
//! ground at y 0), while a submodel's `translate` is its pivot in its parent's frame and its
//! boxes are relative to that pivot. A part keeps the pivot of the vanilla part it replaces
//! (OptiDocs, "Limitations: pivot points"), so a part's `translate` only shifts its content.

use super::json::Json;
use crate::paths;
use std::collections::HashMap;

/// One face of a box, as `horizon_client::mobs::Polygon` holds it: `[x, y, z, u, v]` vertices
/// in model pixels, UV a fraction of the texture, and the outward normal.
#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub normal: [f32; 3],
    pub vertices: [[f32; 5]; 4],
}

/// A baked part model or submodel: a `ModelPart` with its boxes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bone {
    pub id: Option<String>,
    /// `ModelPart` x, y, z: the pivot in the parent's frame, vanilla pixels. Zero for a part,
    /// which keeps its vanilla part's pivot.
    pub pivot: [f32; 3],
    /// `ModelPart` xRot, yRot, zRot in radians, on vanilla's axes.
    pub rotation: [f32; 3],
    pub quads: Vec<Quad>,
    pub children: Vec<Bone>,
}

/// One entry of a `.jem`'s `models`.
#[derive(Clone, Debug, PartialEq)]
pub struct PartModel {
    /// The OptiFine part name it attaches to (`head`, `leg1`...).
    pub part: String,
    /// True: drawn beside the vanilla part's own boxes; false: in place of them.
    pub attach: bool,
    /// `scale`: the render scale of the part.
    pub scale: f32,
    /// `animations`: target and expression, in file order (later ones see earlier results).
    pub animations: Vec<(String, String)>,
    pub bone: Bone,
}

/// A parsed `.jem`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntityModel {
    /// The texture every part draws with; None: the vanilla texture.
    pub texture: Option<String>,
    pub parts: Vec<PartModel>,
    /// What was left out and why: entries whose `.jpm` is missing or broken, a `baseId` naming
    /// no model, past the limits.
    pub warnings: Vec<String>,
}

/// OptiFine's texture size when a model gives none.
const DEFAULT_TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
/// The most entries and quads one `.jem` builds: a pack can't make loading run out of memory.
const MAX_PARTS: usize = 1024;
const MAX_QUADS: usize = 1 << 16;

type Object = Vec<(String, Json)>;

/// Parses the `.jem` at resource path `path` (`assets/minecraft/optifine/cem/zombie.jem`).
/// `read` returns the text of a resource path (the `.jpm` files `model` names), None when missing.
/// An error means the file as a whole can't be read; a broken entry only adds a warning.
pub fn parse_jem(
    path: &str,
    text: &str,
    read: &mut dyn FnMut(&str) -> Option<String>,
) -> Result<EntityModel, String> {
    let root = Json::parse(text).map_err(|e| format!("{path}: {e}"))?;
    let size = size2(root.get("textureSize")).unwrap_or(DEFAULT_TEXTURE_SIZE);
    let texture = root
        .get("texture")
        .and_then(Json::as_str)
        .and_then(|t| texture(path, t));
    let mut out = EntityModel {
        texture,
        ..EntityModel::default()
    };
    let mut jpms: HashMap<String, Result<Object, String>> = HashMap::new();
    let mut by_id: Vec<(String, Object)> = Vec::new();
    let mut budget = MAX_QUADS;
    for entry in objects(root.get("models")) {
        let mut entry = entry.to_vec();
        // `model`: the part model in a `.jpm`, under this entry's own keys.
        if let Some(model) = take(&mut entry, "model") {
            let jpm = model
                .as_str()
                .ok_or_else(|| format!("{path}: a model that isn't a file name"))
                .and_then(|m| jpm(path, m, read, &mut jpms));
            match jpm {
                Ok(jpm) => entry = merged(jpm, entry),
                Err(e) => {
                    out.warnings.push(e);
                    continue;
                }
            }
        }
        // `baseId`: the named entry's properties, under this one's and its `.jpm`'s.
        if let Some(base) = get(&entry, "baseId").and_then(Json::as_str) {
            match by_id.iter().find(|(id, _)| id == base) {
                Some((_, parent)) => entry = merged(parent.clone(), entry),
                None => out
                    .warnings
                    .push(format!("{path}: no model {base} for baseId")),
            }
        }
        if let Some(id) = get(&entry, "id").and_then(Json::as_str) {
            by_id.push((id.to_owned(), entry.clone()));
        }
        let Some(part) = get(&entry, "part").and_then(Json::as_str) else {
            continue;
        };
        if out.parts.len() == MAX_PARTS {
            out.warnings
                .push(format!("{path}: more than {MAX_PARTS} models"));
            break;
        }
        let animations = objects(get(&entry, "animations"))
            .flatten()
            .filter_map(|(k, v)| {
                let expr = match v {
                    Json::Str(s) => s.clone(),
                    Json::Num(n) => n.to_string(),
                    Json::Bool(b) => b.to_string(),
                    _ => return None,
                };
                Some((k.clone(), expr))
            })
            .collect();
        let bone = bone(&entry, size, true, &mut budget);
        out.parts.push(PartModel {
            part: part.to_owned(),
            attach: boolean(get(&entry, "attach")).unwrap_or(false),
            scale: num(get(&entry, "scale")).unwrap_or(1.0),
            animations,
            bone,
        });
    }
    if budget == 0 {
        out.warnings.push(format!(
            "{path}: more than {MAX_QUADS} faces, the rest left out"
        ));
    }
    Ok(out)
}

/// The `.jpm` that `model` (in the `.jem` at `path`) names, read and parsed once per file.
fn jpm(
    path: &str,
    model: &str,
    read: &mut dyn FnMut(&str) -> Option<String>,
    cache: &mut HashMap<String, Result<Object, String>>,
) -> Result<Object, String> {
    let Some(file) = paths::resolve(path, model) else {
        return Err(format!("{path}: {model} is outside the pack"));
    };
    cache
        .entry(file.clone())
        .or_insert_with(|| {
            let text = read(&file).ok_or_else(|| format!("{path}: missing {file}"))?;
            match Json::parse(&text).map_err(|e| format!("{file}: {e}"))? {
                Json::Obj(pairs) => Ok(pairs),
                _ => Err(format!("{file}: not an object")),
            }
        })
        .clone()
}

/// `under`'s keys, with `over`'s in their place where both have one.
fn merged(mut under: Object, over: Object) -> Object {
    under.retain(|(k, _)| !over.iter().any(|(o, _)| o == k));
    under.extend(over);
    under
}

fn get<'a>(object: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    object.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn take(object: &mut Object, key: &str) -> Option<Json> {
    let i = object.iter().rposition(|(k, _)| k == key)?;
    let value = object.remove(i).1;
    object.retain(|(k, _)| k != key);
    Some(value)
}

/// The objects of an array value.
fn objects(v: Option<&Json>) -> impl Iterator<Item = &[(String, Json)]> {
    v.and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(Json::as_object)
}

/// A texture value as a resource path with its `.png`; None for one outside the pack.
fn texture(path: &str, value: &str) -> Option<String> {
    let resolved = paths::resolve(path, value)?;
    Some(if resolved.ends_with(".png") {
        resolved
    } else {
        resolved + ".png"
    })
}

/// A finite number, also when written as a string.
fn num(v: Option<&Json>) -> Option<f32> {
    let n = match v? {
        Json::Num(n) => *n as f32,
        Json::Str(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    n.is_finite().then_some(n)
}

fn boolean(v: Option<&Json>) -> Option<bool> {
    match v? {
        Json::Bool(b) => Some(*b),
        Json::Str(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn nums<const N: usize>(v: Option<&Json>) -> Option<[f32; N]> {
    let a = v?.as_array()?;
    (a.len() >= N).then(|| std::array::from_fn(|i| num(a.get(i)).unwrap_or(0.0)))
}

fn size2(v: Option<&Json>) -> Option<[f32; 2]> {
    nums::<2>(v).filter(|s| s[0] > 0.0 && s[1] > 0.0)
}

/// Bakes a part model definition. `top`: a part (its `translate` shifts its content) rather
/// than a submodel (its `translate` is its pivot). `budget`: the faces still allowed.
fn bone(def: &[(String, Json)], size: [f32; 2], top: bool, budget: &mut usize) -> Bone {
    let axes = get(def, "invertAxis").and_then(Json::as_str).unwrap_or("");
    let invert = [axes.contains('x'), axes.contains('y'), axes.contains('z')];
    let sign = |i: usize| if invert[i] { -1.0 } else { 1.0 };
    let size = size2(get(def, "textureSize")).unwrap_or(size);
    let mirror = get(def, "mirrorTexture")
        .and_then(Json::as_str)
        .unwrap_or("");
    let translate = nums::<3>(get(def, "translate")).unwrap_or_default();
    let translate: [f32; 3] = std::array::from_fn(|i| sign(i) * translate[i]);
    // A rotation seen through flipped axes: about a flipped axis it turns the other way, and an
    // odd number of flips (a mirror) turns every rotation around.
    let det = sign(0) * sign(1) * sign(2);
    let rotate = nums::<3>(get(def, "rotate")).unwrap_or_default();
    let (pivot, offset) = if top {
        ([0.0; 3], translate)
    } else {
        (translate, [0.0; 3])
    };
    let mut out = Bone {
        id: get(def, "id").and_then(Json::as_str).map(str::to_owned),
        pivot,
        rotation: std::array::from_fn(|i| det * sign(i) * rotate[i].to_radians()),
        ..Bone::default()
    };
    let shape = Shape {
        invert,
        mirror_u: mirror.contains('u'),
        mirror_v: mirror.contains('v'),
        size,
        offset,
    };
    for b in objects(get(def, "boxes")) {
        if *budget < 6 {
            *budget = 0;
            break;
        }
        let before = out.quads.len();
        shape.bake(b, &mut out.quads);
        *budget -= out.quads.len() - before;
    }
    let one = get(def, "submodel").and_then(Json::as_object).into_iter();
    for sub in one.chain(objects(get(def, "submodels"))) {
        let mut child = bone(sub, size, false, budget);
        child.pivot = std::array::from_fn(|i| child.pivot[i] + offset[i]);
        out.children.push(child);
    }
    out
}

struct Shape {
    invert: [bool; 3],
    mirror_u: bool,
    mirror_v: bool,
    size: [f32; 2],
    offset: [f32; 3],
}

/// A box's faces in `ModelPart.Cube`'s order of corners, keyed by the per-face UV name for the
/// files' frame (Blockbench's: up is vanilla's -y, east vanilla's -x, north -z).
const FACES: [(&str, &str, [usize; 4], [f32; 3]); 6] = [
    ("uvUp", "", [5, 4, 0, 1], [0.0, -1.0, 0.0]),
    ("uvDown", "", [2, 3, 7, 6], [0.0, 1.0, 0.0]),
    ("uvEast", "uvRight", [0, 4, 7, 3], [-1.0, 0.0, 0.0]),
    ("uvNorth", "uvFront", [1, 0, 3, 2], [0.0, 0.0, -1.0]),
    ("uvWest", "uvLeft", [5, 1, 2, 6], [1.0, 0.0, 0.0]),
    ("uvSouth", "uvBack", [4, 5, 6, 7], [0.0, 0.0, 1.0]),
];
/// `ModelPart.Cube`'s corners: which of x, y, z take the box's max.
const CORNER: [[bool; 3]; 8] = [
    [false, false, false],
    [true, false, false],
    [true, true, false],
    [false, true, false],
    [false, false, true],
    [true, false, true],
    [true, true, true],
    [false, true, true],
];

impl Shape {
    fn bake(&self, b: &[(String, Json)], out: &mut Vec<Quad>) {
        let Some(c) = nums::<6>(get(b, "coordinates")) else {
            return;
        };
        // `sizeAddX/Y/Z` over `sizesAdd` over `sizeAdd`.
        let all = num(get(b, "sizeAdd")).unwrap_or(0.0);
        let mut grow = nums::<3>(get(b, "sizesAdd")).unwrap_or([all; 3]);
        for (i, key) in ["sizeAddX", "sizeAddY", "sizeAddZ"].iter().enumerate() {
            if let Some(g) = num(get(b, key)) {
                grow[i] = g;
            }
        }
        let dims = [c[3], c[4], c[5]];
        let mut lo = [0.0; 3];
        let mut hi = [0.0; 3];
        for i in 0..3 {
            let min = if self.invert[i] {
                -(c[i] + dims[i])
            } else {
                c[i]
            } + self.offset[i];
            lo[i] = min - grow[i];
            hi[i] = min + dims[i] + grow[i];
        }
        let boxed = nums::<2>(get(b, "textureOffset"));
        // A box-UV box mirrored in u is `ModelPart.Cube` with `mirror`: its x sides trade
        // places, and each face winds the other way to keep facing out. Per-face UVs mirror
        // each face's rectangle instead.
        let box_mirror = boxed.is_some() && self.mirror_u;
        let face_mirror_u = boxed.is_none() && self.mirror_u;
        if box_mirror {
            std::mem::swap(&mut lo[0], &mut hi[0]);
        }
        let corner = |i: usize| -> [f32; 3] {
            std::array::from_fn(|a| if CORNER[i][a] { hi[a] } else { lo[a] })
        };
        let rects = match boxed {
            Some(t) => box_rects(t, dims).map(Some),
            None => FACES.map(|(key, alias, ..)| {
                nums::<4>(get(b, key).or_else(|| get(b, alias)))
                    .filter(|r| r[0] != r[2] || r[1] != r[3])
            }),
        };
        for ((_, _, corners, normal), rect) in FACES.iter().zip(rects) {
            let Some(mut r) = rect else {
                continue;
            };
            if face_mirror_u {
                r.swap(0, 2);
            }
            if self.mirror_v {
                r.swap(1, 3);
            }
            let [u1, v1, u2, v2] = r;
            let uv = [(u2, v1), (u1, v1), (u1, v2), (u2, v2)];
            let mut vertices: [[f32; 5]; 4] = std::array::from_fn(|k| {
                let [x, y, z] = corner(corners[k]);
                [x, y, z, uv[k].0 / self.size[0], uv[k].1 / self.size[1]]
            });
            let mut normal = *normal;
            if box_mirror {
                vertices.reverse();
                normal[0] = -normal[0];
            }
            out.push(Quad { normal, vertices });
        }
    }
}

/// `ModelPart.Cube`'s box UV: the faces' `[u1, v1, u2, v2]` around `textureOffset`, in
/// `FACES` order (top strip: up then down; then east, north, west, south).
fn box_rects([u, v]: [f32; 2], [w, h, d]: [f32; 3]) -> [[f32; 4]; 6] {
    let (u1, u2, u3, u4, u5) = (
        u + d,
        u + d + w,
        u + d + w + w,
        u + d + w + d,
        u + 2.0 * (d + w),
    );
    let (v1, v2) = (v + d, v + d + h);
    [
        [u1, v, u2, v1],
        [u2, v1, u3, v],
        [u, v1, u1, v2],
        [u1, v1, u2, v2],
        [u2, v1, u4, v2],
        [u4, v1, u5, v2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const JEM: &str = "assets/minecraft/optifine/cem/zombie.jem";

    fn parse(text: &str) -> EntityModel {
        parse_jem(JEM, text, &mut |_| None).unwrap()
    }
    fn face(bone: &Bone, normal: [f32; 3]) -> &Quad {
        bone.quads.iter().find(|q| q.normal == normal).unwrap()
    }
    fn positions(q: &Quad) -> [[f32; 3]; 4] {
        q.vertices.map(|v| [v[0], v[1], v[2]])
    }

    // The zombie's head and right leg as an export writes them: vanilla's head has its pivot at
    // 0 and a box from (-4, -8, -4); the right leg its pivot at (-1.9, 12, 0) and a box from
    // (-2, 0, -2), 4 x 12 x 4.
    const ZOMBIE: &str = r#"{
        "textureSize": [64, 64],
        "models": [
            {"part": "head", "id": "head", "invertAxis": "xy", "translate": [0, -24, 0],
             "boxes": [{"coordinates": [-4, 24, -4, 8, 8, 8], "textureOffset": [0, 0]}]},
            {"part": "right_leg", "id": "right_leg", "invertAxis": "xy", "translate": [-1.9, -12, 0],
             "boxes": [{"coordinates": [-0.1, 0, -2, 4, 12, 4], "textureOffset": [0, 16]}]}
        ]
    }"#;

    #[test]
    fn parts_land_where_vanilla_has_them_relative_to_their_pivots() {
        let model = parse(ZOMBIE);
        let head = &model.parts[0].bone;
        assert_eq!(head.pivot, [0.0; 3]);
        // The face, at the head's front (-z), with the skin's face square (8, 8)-(16, 16).
        let north = face(head, [0.0, 0.0, -1.0]);
        assert_eq!(
            positions(north),
            [
                [4.0, -8.0, -4.0],
                [-4.0, -8.0, -4.0],
                [-4.0, 0.0, -4.0],
                [4.0, 0.0, -4.0]
            ]
        );
        let uv = north.vertices.map(|v| [v[3] * 64.0, v[4] * 64.0]);
        assert_eq!(uv, [[16.0, 8.0], [8.0, 8.0], [8.0, 16.0], [16.0, 16.0]]);
        // The leg's box spans x -2..2 and y 0..12 around the vanilla pivot.
        let leg = &model.parts[1].bone;
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in leg.quads.iter().flat_map(|q| q.vertices) {
            for a in 0..3 {
                lo[a] = lo[a].min(v[a]);
                hi[a] = hi[a].max(v[a]);
            }
        }
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
        assert!(
            close(lo, [-2.0, 0.0, -2.0]) && close(hi, [2.0, 12.0, 2.0]),
            "{lo:?} {hi:?}"
        );
        // Leg texture: the top at (4, 16)-(8, 20), the west side (vanilla +x) at (8, 20)-(12, 32).
        let uv = |q: &Quad| q.vertices.map(|v| [v[3] * 64.0, v[4] * 64.0]);
        assert_eq!(uv(face(leg, [0.0, -1.0, 0.0]))[1], [4.0, 16.0]);
        assert_eq!(uv(face(leg, [0.0, -1.0, 0.0]))[3], [8.0, 20.0]);
        assert_eq!(uv(face(leg, [1.0, 0.0, 0.0]))[1], [8.0, 20.0]);
        assert_eq!(uv(face(leg, [1.0, 0.0, 0.0]))[3], [12.0, 32.0]);
    }

    #[test]
    fn faces_wind_outward_mirrored_or_not() {
        for mirror in ["", "u"] {
            let model = parse(&format!(
                r#"{{"models": [{{"part": "body", "mirrorTexture": "{mirror}",
                    "boxes": [{{"coordinates": [1, 2, 3, 4, 5, 6], "textureOffset": [0, 0]}}]}}]}}"#
            ));
            let quads = &model.parts[0].bone.quads;
            assert_eq!(quads.len(), 6);
            let mut normals: Vec<[f32; 3]> = quads.iter().map(|q| q.normal).collect();
            normals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            normals.dedup();
            assert_eq!(normals.len(), 6, "{mirror}: every side once");
            for q in quads {
                let [a, b, c, _] = positions(q);
                let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let cross = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let dot: f32 = cross.iter().zip(q.normal).map(|(x, n)| x * n).sum();
                assert!(dot > 0.0, "{mirror}: {q:?}");
                // The face sits on the side its normal points to (box 1..5, 2..7, 3..9).
                let axis = q.normal.iter().position(|n| *n != 0.0).unwrap();
                let side = if q.normal[axis] > 0.0 {
                    [5.0, 7.0, 9.0]
                } else {
                    [1.0, 2.0, 3.0]
                };
                assert!(
                    q.vertices.iter().all(|v| v[axis] == side[axis]),
                    "{mirror}: {q:?}"
                );
            }
        }
    }

    #[test]
    fn mirrored_box_puts_the_first_strip_on_the_other_side() {
        let model = parse(
            r#"{"models": [{"part": "body", "mirrorTexture": "u",
                "boxes": [{"coordinates": [0, 0, 0, 2, 2, 2], "textureOffset": [0, 0]}]}]}"#,
        );
        // Unmirrored, (0, 2)-(2, 4) is the -x side; mirrored it is the +x side.
        let east = face(&model.parts[0].bone, [1.0, 0.0, 0.0]);
        let us: Vec<f32> = east.vertices.iter().map(|v| v[3] * 64.0).collect();
        assert!(us.iter().all(|u| (0.0..=2.0).contains(u)), "{us:?}");
    }

    #[test]
    fn submodels_pivot_in_their_parents_frame_and_rotate_on_vanilla_axes() {
        // A brim on the head pivoting at the head's pivot, and a feather 2 px above it.
        let model = parse(
            r#"{"models": [{"part": "head", "invertAxis": "xy", "translate": [0, -24, 0],
                "submodels": [{"id": "brim", "invertAxis": "xy", "translate": [0, 24, 0],
                    "rotate": [90, 0, 45],
                    "boxes": [{"coordinates": [-1, 0, -1, 2, 2, 2], "textureOffset": [0, 0]}],
                    "submodels": [{"id": "feather", "invertAxis": "xy", "translate": [0, 2, 0]}]}]}]}"#,
        );
        let brim = &model.parts[0].bone.children[0];
        assert_eq!(brim.id.as_deref(), Some("brim"));
        assert_eq!(brim.pivot, [0.0, 0.0, 0.0]);
        let r = std::f32::consts::FRAC_PI_2;
        assert_eq!(brim.rotation, [-r, 0.0, r / 2.0]);
        let ys: Vec<f32> = brim
            .quads
            .iter()
            .flat_map(|q| q.vertices.map(|v| v[1]))
            .collect();
        assert!(ys.iter().all(|y| (-2.0..=0.0).contains(y)), "{ys:?}");
        assert_eq!(brim.children[0].pivot, [0.0, -2.0, 0.0]);
    }

    fn uvs(q: &Quad, w: f32, h: f32) -> [[f32; 2]; 4] {
        q.vertices.map(|v| [v[3] * w, v[4] * h])
    }
    fn bounds(bone: &Bone) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in bone.quads.iter().flat_map(|q| q.vertices) {
            for a in 0..3 {
                lo[a] = lo[a].min(v[a]);
                hi[a] = hi[a].max(v[a]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn each_inverted_axis_flips_position_and_rotation() {
        let part = |axes: &str| {
            parse(&format!(
                r#"{{"models": [{{"part": "body", "invertAxis": "{axes}",
                    "translate": [1, 2, 3], "rotate": [30, 60, 90],
                    "boxes": [{{"coordinates": [1, 2, 3, 1, 1, 1], "textureOffset": [0, 0]}}]}}]}}"#
            ))
            .parts
            .remove(0)
            .bone
        };
        assert_eq!(
            bounds(&part("xyz")),
            ([-3.0, -5.0, -7.0], [-2.0, -4.0, -6.0])
        );
        assert_eq!(bounds(&part("")), ([2.0, 4.0, 6.0], [3.0, 5.0, 7.0]));
        // Degrees to radians; a flipped axis turns the other way, and a mirror (odd flips)
        // turns them all around.
        let deg = |r: [f32; 3]| r.map(|a| (a.to_degrees() * 1000.0).round() / 1000.0);
        assert_eq!(deg(part("").rotation), [30.0, 60.0, 90.0]);
        assert_eq!(deg(part("xy").rotation), [-30.0, -60.0, 90.0]);
        assert_eq!(deg(part("xyz").rotation), [30.0, 60.0, 90.0]);
        assert_eq!(deg(part("x").rotation), [30.0, -60.0, -90.0]);
    }

    #[test]
    fn per_face_uvs_by_name_and_mirrored() {
        let face_of = |key: &str, mirror: &str| {
            let model = parse(&format!(
                r#"{{"models": [{{"part": "body", "textureSize": [16, 16], "mirrorTexture": "{mirror}",
                    "boxes": [{{"coordinates": [0, 0, 0, 2, 2, 2], "{key}": [0, 0, 2, 2]}}]}}]}}"#
            ));
            let quads = &model.parts[0].bone.quads;
            assert_eq!(quads.len(), 1, "{key}");
            quads[0].clone()
        };
        // `uvRight` is `uvEast`: vanilla's -x side.
        let east = face_of("uvRight", "");
        assert_eq!(east.normal, [-1.0, 0.0, 0.0]);
        assert_eq!(
            positions(&east),
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 2.0],
                [0.0, 2.0, 2.0],
                [0.0, 2.0, 0.0]
            ]
        );
        assert_eq!(
            uvs(&east, 16.0, 16.0),
            [[2.0, 0.0], [0.0, 0.0], [0.0, 2.0], [2.0, 2.0]]
        );
        assert_eq!(
            uvs(&face_of("uvRight", "uv"), 16.0, 16.0),
            [[0.0, 2.0], [2.0, 2.0], [2.0, 0.0], [0.0, 0.0]]
        );
        let west = face_of("uvLeft", "");
        assert_eq!(west.normal, [1.0, 0.0, 0.0]);
        assert!(west.vertices.iter().all(|v| v[0] == 2.0));
    }

    #[test]
    fn size_adds_grow_positions_per_axis() {
        let model = parse(
            r#"{"models": [{"part": "body", "boxes": [{"coordinates": [0, 0, 0, 2, 2, 2],
                "textureOffset": [0, 0], "sizeAdd": 9, "sizesAdd": [1, 0, 0], "sizeAddY": 0.5}]}]}"#,
        );
        let bone = &model.parts[0].bone;
        assert_eq!(bounds(bone), ([-1.0, -0.5, 0.0], [3.0, 2.5, 2.0]));
        // The UVs keep the box's own size: the north face spans u 2..4 of the default 64 x 32.
        assert_eq!(
            uvs(face(bone, [0.0, 0.0, -1.0]), 64.0, 32.0),
            [[4.0, 2.0], [2.0, 2.0], [2.0, 4.0], [4.0, 4.0]]
        );
    }

    #[test]
    fn texture_size_reaches_a_single_submodel() {
        let model = parse(
            r#"{"models": [{"part": "head", "invertAxis": "xy", "translate": [0, -24, 0], "textureSize": [16, 16],
                "submodel": {"boxes": [{"coordinates": [0, 0, 0, 1, 1, 1], "uvUp": [0, 0, 1, 1]}]}}]}"#,
        );
        let child = &model.parts[0].bone.children[0];
        assert_eq!(child.pivot, [0.0, 24.0, 0.0]);
        let max_u = child.quads[0]
            .vertices
            .iter()
            .map(|v| v[3])
            .fold(0.0, f32::max);
        assert_eq!(max_u, 1.0 / 16.0);
    }

    #[test]
    fn animations_keep_file_order() {
        let model = parse(
            r#"{"models": [{"part": "head", "attach": "true",
                "animations": [{"var.a": "1", "head.rx": "var.a"}, {"this.tx": 0}]}]}"#,
        );
        let head = &model.parts[0];
        assert!(head.attach);
        assert_eq!(
            head.animations,
            [("var.a", "1"), ("head.rx", "var.a"), ("this.tx", "0")]
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
        );
    }

    #[test]
    fn part_files_and_bases() {
        let file = |boxes: &str| {
            format!(
                r#"{{"textureSize": [32, 32], "boxes": [{{"coordinates": [{boxes}], "uvFront": [0, 0, 4, 2]}}]}}"#
            )
        };
        let mut asked = Vec::new();
        let model = parse_jem(
            JEM,
            r#"{"texture": "textures/entity/zombie/green", "models": [
                {"id": "base", "part": "head", "model": "hat.jpm", "scale": 1.5},
                {"baseId": "base", "part": "headwear", "attach": true},
                {"baseId": "base", "part": "body", "model": "./parts/vest.jpm"},
                {"part": "left_arm", "model": "hat.jpm"}
            ]}"#,
            &mut |p| {
                asked.push(p.to_owned());
                Some(file(if p.ends_with("vest.jpm") {
                    "0, 0, 0, 8, 8, 8"
                } else {
                    "0, 0, 0, 4, 2, 4"
                }))
            },
        )
        .unwrap();
        // Each file is read once, however many entries name it.
        assert_eq!(
            asked,
            [
                "assets/minecraft/optifine/cem/hat.jpm",
                "assets/minecraft/optifine/cem/parts/vest.jpm"
            ]
        );
        assert_eq!(
            model.texture.as_deref(),
            Some("assets/minecraft/textures/entity/zombie/green.png")
        );
        let [head, headwear, body, arm] = &model.parts[..] else {
            panic!("{:?}", model.parts);
        };
        assert_eq!((head.scale, head.attach), (1.5, false));
        assert_eq!(head.bone.quads.len(), 1);
        assert_eq!(bounds(&head.bone), ([0.0, 0.0, 0.0], [4.0, 2.0, 0.0]));
        // The second entry inherits the first's part model under its own part and attach.
        assert_eq!(
            (headwear.part.as_str(), headwear.attach, headwear.scale),
            ("headwear", true, 1.5)
        );
        assert_eq!(headwear.bone.quads, head.bone.quads);
        // Its own part file wins over the inherited one.
        assert_eq!(bounds(&body.bone), ([0.0, 0.0, 0.0], [8.0, 8.0, 0.0]));
        assert_eq!(arm.bone.quads, head.bone.quads);
        assert!(model.warnings.is_empty(), "{:?}", model.warnings);
    }

    #[test]
    fn broken_entries_are_left_out_with_a_warning() {
        let mut asked = Vec::new();
        let model = parse_jem(
            JEM,
            "\u{feff}{\"models\": [ // a hand-edited file\n
                {\"part\": \"head\", \"model\": \"missing.jpm\"},
                {\"part\": \"head\", \"model\": \"../../../../etc/passwd\"},
                {\"part\": \"body\", \"baseId\": \"nope\", \"boxes\": [{\"coordinates\": [0, 0, 0, 1, 1, 1], \"textureOffset\": [0, 0]}]},
                {\"part\": \"tail\", \"scale\": \"NaN\"},
            ]}",
            &mut |p| {
                asked.push(p.to_owned());
                None
            },
        )
        .unwrap();
        assert_eq!(asked, ["assets/minecraft/optifine/cem/missing.jpm"]);
        let names: Vec<&str> = model.parts.iter().map(|p| p.part.as_str()).collect();
        assert_eq!(names, ["body", "tail"]);
        assert_eq!(model.parts[0].bone.quads.len(), 6);
        assert_eq!(model.parts[1].scale, 1.0);
        assert_eq!(model.warnings.len(), 3, "{:?}", model.warnings);
        assert!(model.warnings[1].contains("outside the pack"));
        assert!(parse_jem(JEM, "{", &mut |_| None).is_err());
    }

    #[test]
    fn a_file_builds_at_most_the_face_limit() {
        let one = r#"{"coordinates": [0, 0, 0, 1, 1, 1], "textureOffset": [0, 0]}"#;
        let boxes = vec![one; MAX_QUADS / 6 + 10].join(",");
        let model = parse(&format!(
            r#"{{"models": [{{"part": "body", "boxes": [{boxes}]}}]}}"#
        ));
        assert_eq!(model.parts[0].bone.quads.len(), MAX_QUADS / 6 * 6);
        assert_eq!(model.warnings.len(), 1);
    }
}
