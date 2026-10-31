//! Paths in OptiFine `.properties` files, and the folders that hold those files. Paths here are
//! resource paths inside the overlay resource root (`assets/<namespace>/...`, `/`-separated),
//! the tree `horizon_client::packs::resource_root` builds from the enabled packs.

use std::fs;
use std::path::Path;

/// The resource path a value of the properties file at `props` names:
/// - `~/x`: `assets/minecraft/optifine/x`;
/// - `namespace:x`: `assets/namespace/x`;
/// - `./x`, or a bare name without `/`: beside the properties file;
/// - any other `a/b`: `assets/<the file's namespace>/a/b`.
///
/// The extension is the caller's: OptiFine adds `.png` to textures that have none.
pub fn resolve(props: &str, value: &str) -> String {
    let value = value.trim();
    let dir = props.rsplit_once('/').map_or("", |(dir, _)| dir);
    if let Some(rest) = value.strip_prefix("~/") {
        return format!("assets/minecraft/optifine/{rest}");
    }
    if let Some(rest) = value.strip_prefix("./") {
        return format!("{dir}/{rest}");
    }
    if let Some((ns, rest)) = value.split_once(':') {
        return format!("assets/{ns}/{rest}");
    }
    if !value.contains('/') {
        return format!("{dir}/{value}");
    }
    let namespace = props
        .strip_prefix("assets/")
        .and_then(|p| p.split('/').next())
        .unwrap_or("minecraft");
    format!("assets/{namespace}/{value}")
}

/// Every `.properties` file under `assets/*/optifine/<feature>/` and the legacy
/// `assets/*/mcpatcher/<feature>/` of `root`, as sorted resource paths. `feature` is a folder
/// (`ctm`, `cit`, `random/entity`, `sky/world0`).
pub fn properties_files(root: &Path, feature: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(namespaces) = fs::read_dir(root.join("assets")) else {
        return out;
    };
    for ns in namespaces.flatten() {
        let ns = ns.file_name().to_string_lossy().into_owned();
        for top in ["optifine", "mcpatcher"] {
            let base = format!("assets/{ns}/{top}/{feature}");
            walk(&root.join(&base), &base, &mut out);
        }
    }
    out.sort();
    out
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = format!("{rel}/{name}");
        // `metadata` follows links: overlay folders are links into the packs and vanilla.
        match fs::metadata(e.path()) {
            Ok(m) if m.is_dir() => walk(&e.path(), &rel, out),
            Ok(_) if name.ends_with(".properties") => out.push(rel),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolving() {
        let props = "assets/minecraft/optifine/cit/swords/flame.properties";
        let r = |v| resolve(props, v);
        assert_eq!(r("flame"), "assets/minecraft/optifine/cit/swords/flame");
        assert_eq!(
            r("./sub/flame.png"),
            "assets/minecraft/optifine/cit/swords/sub/flame.png"
        );
        assert_eq!(
            r("~/cit/shared/glow.png"),
            "assets/minecraft/optifine/cit/shared/glow.png"
        );
        assert_eq!(
            r("textures/item/stick.png"),
            "assets/minecraft/textures/item/stick.png"
        );
        assert_eq!(r("mymod:textures/x.png"), "assets/mymod/textures/x.png");
        assert_eq!(
            resolve(
                "assets/mymod/optifine/ctm/a.properties",
                "textures/block/b.png"
            ),
            "assets/mymod/textures/block/b.png"
        );
    }

    #[test]
    fn finds_both_folders_in_every_namespace() {
        let root = std::env::temp_dir().join(format!("horizon-optifine-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for f in [
            "assets/minecraft/optifine/ctm/glass/glass.properties",
            "assets/minecraft/optifine/ctm/glass/0.png",
            "assets/minecraft/mcpatcher/ctm/old.properties",
            "assets/mymod/optifine/ctm/x/y.properties",
            "assets/minecraft/optifine/cit/sword.properties",
        ] {
            let path = root.join(f);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        assert_eq!(
            properties_files(&root, "ctm"),
            [
                "assets/minecraft/mcpatcher/ctm/old.properties",
                "assets/minecraft/optifine/ctm/glass/glass.properties",
                "assets/mymod/optifine/ctm/x/y.properties",
            ]
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
