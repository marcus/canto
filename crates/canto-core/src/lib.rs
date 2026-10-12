//! Canto core: song generation with YuE2, prompt refinement, and the song library.
//!
//! The CLI and the GPUI app are thin shells over [`Studio`]. Engines and
//! refiners sit behind traits so a new model or agent CLI is a new adapter.

pub mod audio;
pub mod config;
pub mod engine;
pub mod library;
pub mod refine;
pub mod song;
pub mod studio;

pub use config::Config;
pub use engine::{CancelToken, Cancelled, Engine, EngineEvent, Progress, Stage};
pub use library::Library;
pub use refine::{Brief, RefineTask, Refiner, RefinerKind, Scope};
pub use song::{DRAFT_STEPS, FULL_STEPS, Mode, Song, SongMeta, SongRequest, SongStatus};
pub use studio::Studio;

/// A random seed without pulling in a RNG crate.
pub fn random_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    h.finish() % 100_000_000
}
