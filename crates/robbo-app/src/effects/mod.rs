//! Visual effects driven by [`CoreGameEvent`] (observer pattern).
//!
//! Simulation stays in `robbo-core`; this module only spawns short-lived
//! presentation entities. Start with teleport bursts; explosion and shot trails
//! use the same [`FxParticle`] pipeline (see [`presets`]).

mod particle;
mod presets;
mod systems;

pub use systems::{fx_on_core_events, tick_fx_particles};
