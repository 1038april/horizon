//! Items' default components, as everything in the game reads them: the
//! resource root's `items.json` (vanilla's default-component report, with a
//! content mod's items), then what native mods change in it
//! ([`native_modify_default_components`], NeoForge's `ModifyDefaultComponentsEvent`).
//! The item table, loot, enchanting, item models and the other readers all
//! come through [`read`], so a mod's change reaches each of them, and stacks
//! are saved and sent against the changed defaults as against vanilla's.

use serde_json::{Map, Value};
use std::{collections::BTreeMap, fs, io, path::Path};

/// Hook point for native mods (docs/native-mods.md, listed in `native-hooks.json`
/// with those of `horizon-client/src/game/native_hooks.rs`): `components` holds
/// `item`'s default components in `items.json`'s form (`"minecraft:max_stack_size": 16`,
/// `"minecraft:food": {"nutrition": 4, "saturation": 2.4}`); insert or replace one to
/// set it, remove it to take it away. Asked for every item each time the game reads
/// the defaults, several times as resources load and again when a world's data packs
/// load: answer the same each time. Client and server each ask their own mods, as
/// NeoForge does, so both sides need the mod. @API:modify_default_components@.
#[inline(never)]
pub fn native_modify_default_components(item: &str, components: &mut Map<String, Value>) {
    let _ = (item, components);
}

/// Every item's default components at `root`, by item id, with the mods'
/// changes applied.
pub fn read(root: &Path) -> io::Result<BTreeMap<String, Value>> {
    let mut items: BTreeMap<String, Value> =
        serde_json::from_slice(&fs::read(root.join("items.json"))?)?;
    for (item, components) in &mut items {
        if let Value::Object(components) = components {
            native_modify_default_components(item, components);
        }
    }
    Ok(items)
}
