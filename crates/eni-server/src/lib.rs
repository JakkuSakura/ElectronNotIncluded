//! In-process authoritative server: chunk management, player movement, time,
//! and the chemistry simulation tick.

mod chunk_manager;
mod fluid;
mod player_movement;
mod simulation;
mod time;

use bevy::prelude::*;
use chunk_manager::{ChunkManager, manage_chunks};
use eni_domain::{GameClock, PlayerPosition};
use player_movement::authoritative_movement;

pub use simulation::tick_chunk;

/// World generation seed for chunks streamed by this server instance.
#[derive(Default)]
pub struct ServerPlugin {
    pub seed: u32,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameClock>()
            .insert_resource(ChunkManager::new(3, self.seed))
            .insert_resource(PlayerPosition { x: 0.0, y: 0.0 })
            .add_systems(Update, time::advance_real_time)
            .add_systems(Update, authoritative_movement)
            .add_systems(Update, manage_chunks)
            .add_systems(FixedUpdate, simulation::advance_simulation);
    }
}
