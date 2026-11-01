//! The files OptiFine-format resource packs share, below every feature that reads them (connected
//! textures, custom item textures, random and emissive entities, custom entity models, sky,
//! colours): `.properties` files, the matchers their values hold, their texture paths, and the
//! `optifine/` (or legacy `mcpatcher/`) folders that hold them. The readers of single features
//! that need no game types live here too: [`ctm`] (connected textures).
//!
//! Written from OptiFine's public documentation of the file formats; no mod's source was read.

pub mod ctm;
pub mod matchers;
pub mod paths;
pub mod properties;

pub use matchers::{BlockMatch, IdList, Name, RangeList};
pub use paths::{properties_files, resolve};
pub use properties::Properties;
