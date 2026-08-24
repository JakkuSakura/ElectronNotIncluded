//! Authoritative player movement over the tile grid.
//!
//! Solid tiles (dominant substance phase `Solid`) block movement; this is a
//! generic tile-grid movement rule, with no cultivation-specific terrain cost.

use bevy::prelude::*;
use eni_domain::{GameData, Phase, PlayerMoveIntent, PlayerPosition, WorldCoord};

use crate::chunk_manager::ChunkManager;

pub const PLAYER_SPEED: f32 = 2.0;
/// Movement is unbounded by a world radius in this pass (no world-meta
/// config exists yet); clamp to a generous default instead.
pub const WORLD_HALF_EXTENT: f32 = 100_000.0;

pub fn authoritative_movement(
    mut player_pos: ResMut<PlayerPosition>,
    chunk_mgr: Res<ChunkManager>,
    game_data: Res<GameData>,
    mut intent_reader: MessageReader<PlayerMoveIntent>,
    mut pos_writer: MessageWriter<PlayerPosition>,
) {
    for intent in intent_reader.read() {
        let movement = intent.direction * PLAYER_SPEED * intent.delta_seconds;

        let new_x = player_pos.x + movement.x;
        let new_y = player_pos.y + movement.y;

        let target_coord = WorldCoord {
            x: new_x.floor() as i32,
            y: new_y.floor() as i32,
        };

        let blocked = chunk_mgr.get_tile(target_coord).is_some_and(|tile| {
            tile.mass_kg
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .and_then(|(id, _)| game_data.substances.get(id))
                .is_some_and(|def| def.phase_at_stp == Phase::Solid)
        });

        if !blocked {
            player_pos.x = new_x.clamp(-WORLD_HALF_EXTENT, WORLD_HALF_EXTENT);
            player_pos.y = new_y.clamp(-WORLD_HALF_EXTENT, WORLD_HALF_EXTENT);
        }
    }
    pos_writer.write(*player_pos);
}
