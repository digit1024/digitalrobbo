//! Visual effects driven by [`CoreGameEvent`] (observer pattern).
//!
//! Simulation stays in `robbo-core`; this module only spawns short-lived
//! presentation entities. See [`docs/architecture-design/11-visual-effects.md`](../../docs/architecture-design/11-visual-effects.md).

mod aura;
mod capsule_visual;
mod collect;
mod magnet;
mod particle;
mod presets;
mod screw_visual;
mod systems;

pub use aura::{sync_fx_auras, tick_teleport_auras};
pub use capsule_visual::{update_capsule_visuals, CapsuleVisual};
pub use collect::tick_collect_pop_effects;
pub use magnet::{clear_magnet_beams_on_reload, update_magnet_beams, update_magnet_visuals, MagnetBeamCache, MagnetVisual};
pub use screw_visual::{ScrewVisual, update_screw_visuals};
pub use systems::{fx_on_core_events, tick_fx_particles};
