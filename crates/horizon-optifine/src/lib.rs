//! The files OptiFine-format resource packs share, below every feature that reads them (connected
//! textures, custom item textures, random and emissive entities, custom entity models, sky,
//! colours): `.properties` files, the matchers their values hold, their texture paths, and the
//! `optifine/` (or legacy `mcpatcher/`) folders that hold them; and the rules of custom item
//! textures (`cit`), which the client matches against its stacks.
//!
//! Written from OptiFine's public documentation of the file formats; `cit` also from CIT Resewn's
//! wiki and the conditions of its MIT-licensed source. No OptiFine code was read.

pub mod cit;
pub mod matchers;
pub mod paths;
pub mod properties;

pub use matchers::{BlockMatch, IdList, Name, RangeList};
pub use paths::{properties_files, resolve, valid};
pub use properties::Properties;
