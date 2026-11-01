//! OptiFine's Custom Item Textures (and CIT Resewn's): the rules of the enabled packs'
//! `optifine/cit/**/*.properties`, read from the overlay resource root. The rules themselves are
//! `horizon_optifine::cit`'s.

use horizon_optifine::cit::Rule;
use horizon_optifine::{Properties, properties_files};
use std::fs;
use std::path::Path;

/// Every rule under the root's `optifine/cit/` and `mcpatcher/cit/` folders, the one chosen
/// first when several match first (`Rule::priority`). The pack's global `cit.properties` is not
/// a rule.
pub fn load_rules(root: &Path) -> Vec<Rule> {
    let mut rules: Vec<Rule> = properties_files(root, "cit")
        .into_iter()
        .filter(|path| !path.ends_with("/cit.properties"))
        .filter_map(|path| {
            let text = fs::read_to_string(root.join(&path)).ok()?;
            let rule = Rule::parse(&path, &Properties::parse(&text), |p| root.join(p).is_file());
            if rule.is_none() {
                log::warn!("custom item textures: {path} is not a rule Horizon can read");
            }
            rule
        })
        .collect();
    rules.sort_by(Rule::priority);
    rules
}
