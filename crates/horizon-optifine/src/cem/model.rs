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

use crate::paths;
use serde_json::{Map, Value};

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
    /// The texture (a resource path ending in `.png`) this bone and its children draw with
    /// instead of the model's.
    pub texture: Option<String>,
    /// `ModelPart` x, y, z: the pivot in the parent's frame, vanilla pixels. Zero for a part,
    /// which keeps its vanilla part's pivot.
    pub pivot: [f32; 3],
    /// `ModelPart` xRot, yRot, zRot in radians, on vanilla's axes.
    pub rotation: [f32; 3],
    pub quads: Vec<Quad>,
    pub children: Vec<Bone>,
    /// `attachments`: where held items (`left_handheld_item`...) render, in this bone's frame.
    pub attachments: Vec<(String, [f32; 3])>,
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
    /// The texture every part draws with unless it names its own; None: the vanilla texture.
    pub texture: Option<String>,
    pub shadow_size: Option<f32>,
    pub parts: Vec<PartModel>,
}

/// OptiFine's texture size when a model gives none.
const DEFAULT_TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];

/// Parses the `.jem` at resource path `path` (`assets/minecraft/optifine/cem/zombie.jem`).
/// `read` returns the text of a resource path (the `.jpm` files `model` names), None when missing.
pub fn parse_jem(
    path: &str,
    text: &str,
    read: &mut dyn FnMut(&str) -> Option<String>,
) -> Result<EntityModel, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("{path}: {e}"))?;
    let size = size2(root.get("textureSize")).unwrap_or(DEFAULT_TEXTURE_SIZE);
    let texture = root
        .get("texture")
        .and_then(Value::as_str)
        .map(|t| texture(path, t));
    let mut by_id: Vec<(String, Map<String, Value>)> = Vec::new();
    let mut parts = Vec::new();
    for entry in root
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(entry) = entry.as_object() else {
            continue;
        };
        let mut entry = entry.clone();
        // `baseId`: the named entry's properties, under this one's.
        if let Some(base) = entry.get("baseId").and_then(Value::as_str)
            && let Some((_, parent)) = by_id.iter().find(|(id, _)| id == base)
        {
            entry = merged(parent.clone(), entry);
        }
        // `model`: the part model from a `.jpm`, under this entry's own keys.
        // Loaded once: an entry that inherits it through `baseId` takes the loaded keys.
        if let Some(Value::String(model)) = entry.remove("model") {
            let jpm_path = paths::resolve(path, &model);
            let jpm = read(&jpm_path).ok_or_else(|| format!("{path}: missing {jpm_path}"))?;
            let jpm: Value = serde_json::from_str(&jpm).map_err(|e| format!("{jpm_path}: {e}"))?;
            if let Some(jpm) = jpm.as_object() {
                entry = merged(jpm.clone(), entry);
            }
        }
        if let Some(id) = entry.get("id").and_then(Value::as_str) {
            by_id.push((id.to_owned(), entry.clone()));
        }
        let Some(part) = entry.get("part").and_then(Value::as_str) else {
            continue;
        };
        let animations = entry
            .get("animations")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
            .flat_map(|group| group.iter())
            .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
            .collect();
        parts.push(PartModel {
            part: part.to_owned(),
            attach: entry
                .get("attach")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            scale: num(entry.get("scale")).unwrap_or(1.0),
            animations,
            bone: bone(path, &entry, size, true),
        });
    }
    Ok(EntityModel {
        texture,
        shadow_size: num(root.get("shadowSize")),
        parts,
    })
}

fn merged(mut under: Map<String, Value>, over: Map<String, Value>) -> Map<String, Value> {
    under.extend(over);
    under
}

/// A texture value as a resource path with its `.png`.
fn texture(path: &str, value: &str) -> String {
    let resolved = paths::resolve(path, value);
    if resolved.ends_with(".png") {
        resolved
    } else {
        resolved + ".png"
    }
}

fn num(v: Option<&Value>) -> Option<f32> {
    match v? {
        Value::Number(n) => n.as_f64().map(|n| n as f32),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn nums<const N: usize>(v: Option<&Value>) -> Option<[f32; N]> {
    let a = v?.as_array()?;
    (a.len() >= N).then(|| std::array::from_fn(|i| num(a.get(i)).unwrap_or(0.0)))
}

fn size2(v: Option<&Value>) -> Option<[f32; 2]> {
    nums::<2>(v).filter(|s| s[0] > 0.0 && s[1] > 0.0)
}

/// Bakes a part model definition. `top`: a part (its `translate` shifts its content) rather
/// than a submodel (its `translate` is its pivot).
fn bone(path: &str, def: &Map<String, Value>, size: [f32; 2], top: bool) -> Bone {
    let axes = def.get("invertAxis").and_then(Value::as_str).unwrap_or("");
    let invert = [axes.contains('x'), axes.contains('y'), axes.contains('z')];
    let sign = |i: usize, v: f32| if invert[i] { -v } else { v };
    let size = size2(def.get("textureSize")).unwrap_or(size);
    let mirror = def
        .get("mirrorTexture")
        .and_then(Value::as_str)
        .unwrap_or("");
    let translate = nums::<3>(def.get("translate")).unwrap_or_default();
    let translate: [f32; 3] = std::array::from_fn(|i| sign(i, translate[i]));
    let rotate = nums::<3>(def.get("rotate")).unwrap_or_default();
    let (pivot, offset) = if top {
        ([0.0; 3], translate)
    } else {
        (translate, [0.0; 3])
    };
    let mut out = Bone {
        id: def.get("id").and_then(Value::as_str).map(str::to_owned),
        texture: def
            .get("texture")
            .and_then(Value::as_str)
            .map(|t| texture(path, t)),
        pivot,
        rotation: std::array::from_fn(|i| sign(i, rotate[i].to_radians())),
        ..Bone::default()
    };
    let shape = Shape {
        invert,
        mirror_u: mirror.contains('u'),
        mirror_v: mirror.contains('v'),
        size,
        offset,
    };
    for b in def
        .get("boxes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(b) = b.as_object() {
            shape.bake(b, &mut out.quads);
        }
    }
    let one = def.get("submodel").into_iter();
    let many = def
        .get("submodels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for sub in one.chain(many).filter_map(Value::as_object) {
        let mut child = bone(path, sub, size, false);
        child.pivot = std::array::from_fn(|i| child.pivot[i] + offset[i]);
        out.children.push(child);
    }
    if let Some(points) = def.get("attachments").and_then(Value::as_object) {
        for (name, at) in points {
            let at = nums::<3>(Some(at)).unwrap_or_default();
            let at = std::array::from_fn(|i| sign(i, at[i]) + offset[i]);
            out.attachments.push((name.clone(), at));
        }
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
    fn bake(&self, b: &Map<String, Value>, out: &mut Vec<Quad>) {
        let Some(c) = nums::<6>(b.get("coordinates")) else {
            return;
        };
        let all = num(b.get("sizeAdd")).unwrap_or(0.0);
        let mut grow = nums::<3>(b.get("sizesAdd")).unwrap_or([all; 3]);
        for (i, key) in ["sizeAddX", "sizeAddY", "sizeAddZ"].iter().enumerate() {
            if let Some(g) = num(b.get(*key)) {
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
        let boxed = nums::<2>(b.get("textureOffset"));
        // `ModelPart.Cube` with `mirror`: the box's x sides trade places, and each face winds
        // the other way to keep facing out.
        let mirror = boxed.is_some() && self.mirror_u;
        if mirror {
            std::mem::swap(&mut lo[0], &mut hi[0]);
        }
        let corner = |i: usize| -> [f32; 3] {
            std::array::from_fn(|a| if CORNER[i][a] { hi[a] } else { lo[a] })
        };
        let rects = match boxed {
            Some(t) => box_rects(t, dims).map(Some),
            None => FACES.map(|(key, alias, ..)| {
                nums::<4>(b.get(key).or_else(|| b.get(alias)))
                    .filter(|r| r[0] != r[2] || r[1] != r[3])
            }),
        };
        for ((_, _, corners, normal), rect) in FACES.iter().zip(rects) {
            let Some(mut r) = rect else {
                continue;
            };
            if self.mirror_u && !mirror {
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
            if mirror {
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

    #[test]
    fn part_files_bases_textures_and_face_uvs() {
        let jpm = r#"{"textureSize": [32, 32], "texture": "hat",
            "boxes": [{"coordinates": [0, 0, 0, 4, 2, 4], "sizeAdd": 0.5,
                       "uvFront": [0, 0, 4, 2], "uvUp": [4, 4, 4, 4]}]}"#;
        let mut asked = Vec::new();
        let model = parse_jem(
            JEM,
            r#"{"texture": "textures/entity/zombie/green", "shadowSize": 0.75, "models": [
                {"id": "base", "part": "head", "model": "hat.jpm", "scale": 1.5,
                 "animations": [{"this.rx": "sin(age)", "var.a": "1"}, {"head.ty": "var.a"}]},
                {"baseId": "base", "part": "headwear", "attach": true}
            ]}"#,
            &mut |p| {
                asked.push(p.to_owned());
                Some(jpm.to_owned())
            },
        )
        .unwrap();
        assert_eq!(asked, ["assets/minecraft/optifine/cem/hat.jpm"]);
        assert_eq!(
            model.texture.as_deref(),
            Some("assets/minecraft/textures/entity/zombie/green.png")
        );
        assert_eq!(model.shadow_size, Some(0.75));
        let head = &model.parts[0];
        assert_eq!(
            head.bone.texture.as_deref(),
            Some("assets/minecraft/optifine/cem/hat.png")
        );
        assert_eq!((head.scale, head.attach), (1.5, false));
        assert_eq!(
            head.animations,
            [
                ("this.rx", "sin(age)"),
                ("var.a", "1"),
                ("head.ty", "var.a")
            ]
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
        );
        // The empty up face draws nothing; the front grows by the sizeAdd, its UV does not.
        let front = &head.bone.quads;
        assert_eq!(front.len(), 1);
        assert_eq!(positions(&front[0])[1], [-0.5, -0.5, -0.5]);
        assert_eq!(front[0].vertices[0][3..], [4.0 / 32.0, 0.0]);
        // The second entry inherits the first's part model under its own part and attach.
        let headwear = &model.parts[1];
        assert_eq!(
            (headwear.part.as_str(), headwear.attach),
            ("headwear", true)
        );
        assert_eq!(headwear.bone.quads, head.bone.quads);
    }

    #[test]
    fn missing_part_files_and_bad_json_are_errors() {
        let missing = parse_jem(
            JEM,
            r#"{"models": [{"part": "head", "model": "x.jpm"}]}"#,
            &mut |_| None,
        );
        assert!(missing.unwrap_err().contains("optifine/cem/x.jpm"));
        assert!(parse_jem(JEM, "{", &mut |_| None).is_err());
    }
}
