//! Shared domain model for ElectronNotIncluded: chunked tile grid, chemistry
//! (substances/reactions/mixing), and time-of-day tracking used by both the
//! server and the client.

mod chemistry;
mod chunk;
mod data;
mod world;

use bevy::prelude::*;

pub use chemistry::{
    Composition, Phase, ReactionRule, SubstanceDefinition, SubstanceId, SubstanceRegistry,
    try_react,
};
pub use chunk::{
    CHUNK_AREA, CHUNK_SIZE, CHUNK_SIZE_U32, ChunkCoord, ChunkData, PlayerMoveIntent,
    PlayerPosition, TileGrid, UnloadChunk, WorldChunk, WorldCoord,
};
pub use data::{DataError, GameData};
pub use world::{generate_chunk, stable_hash};

/// Shared game state, used by both server and client for scheduling.
#[derive(States, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
}

/// Authoritative simulation clock: counts ticks of the chemistry/physics
/// pipeline, independent of the calendar `GameClock` below.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SimulationClock {
    pub tick: u64,
}

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GamePaused(pub bool);

pub const REAL_SECONDS_PER_GAME_DAY: f64 = 300.0;
pub const GAME_SECONDS_PER_DAY: f64 = 86_400.0;
pub const GAME_DAYS_PER_YEAR: u32 = 12;

/// Server-authoritative game clock: 5 minutes of wall time equals 1 game day.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GameClock {
    elapsed_game_seconds: f64,
}

impl Default for GameClock {
    fn default() -> Self {
        let start_hour = 12.0;
        Self {
            elapsed_game_seconds: start_hour * 3600.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameDateTime {
    pub year: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl GameClock {
    pub fn advance_real_seconds(&mut self, real_seconds: f64) {
        if real_seconds > 0.0 {
            self.elapsed_game_seconds +=
                real_seconds * GAME_SECONDS_PER_DAY / REAL_SECONDS_PER_GAME_DAY;
        }
    }

    pub fn date_time(&self) -> GameDateTime {
        let total_seconds = self.elapsed_game_seconds.floor() as u64;
        let day_index = total_seconds / GAME_SECONDS_PER_DAY as u64;
        let day_seconds = total_seconds % GAME_SECONDS_PER_DAY as u64;
        GameDateTime {
            year: (day_index / GAME_DAYS_PER_YEAR as u64 + 1) as u32,
            day: (day_index % GAME_DAYS_PER_YEAR as u64 + 1) as u32,
            hour: (day_seconds / 3_600) as u32,
            minute: ((day_seconds % 3_600) / 60) as u32,
            second: (day_seconds % 60) as u32,
        }
    }
}

/// Movement intent sent from the client to the in-process server.
#[derive(Message, Clone, Copy, Debug)]
pub struct MoveIntent {
    pub direction: IVec2,
}

/// Simulation-advanced broadcast from server to client.
#[derive(Message, Clone, Copy, Debug)]
pub struct SimulationAdvanced {
    pub tick: u64,
}

/// Registers the shared resources and messages that cross the in-process
/// client/server boundary.
pub struct DomainPlugin;

impl Plugin for DomainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationClock>()
            .init_resource::<GamePaused>()
            .add_message::<MoveIntent>()
            .add_message::<SimulationAdvanced>()
            .add_message::<ChunkData>()
            .add_message::<UnloadChunk>()
            .add_message::<PlayerMoveIntent>()
            .add_message::<PlayerPosition>();
    }
}

#[cfg(test)]
mod time_tests {
    use super::*;

    #[test]
    fn wall_time_maps_to_game_calendar() {
        let mut clock = GameClock::default();
        clock.advance_real_seconds(REAL_SECONDS_PER_GAME_DAY);
        assert_eq!(
            clock.date_time(),
            GameDateTime {
                year: 1,
                day: 2,
                hour: 12,
                minute: 0,
                second: 0,
            }
        );
        clock.advance_real_seconds(REAL_SECONDS_PER_GAME_DAY * 11.0);
        assert_eq!(clock.date_time().year, 2);
        assert_eq!(clock.date_time().day, 1);
    }

    #[test]
    fn game_clock_tracks_in_day_time() {
        let mut clock = GameClock::default();
        clock.advance_real_seconds(12.5);
        assert_eq!(clock.date_time().hour, 13);
        assert_eq!(clock.date_time().minute, 0);
        assert_eq!(clock.date_time().second, 0);
    }
}
