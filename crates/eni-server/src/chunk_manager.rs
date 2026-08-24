//! Chunk loading, unloading, and streaming to the client.
//!
//! Only a single in-memory chunk radius is implemented in this pass;
//! multi-chunk streaming across a persistent world is future work (see the
//! `todo!()` below).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use eni_domain::{
    ChunkCoord, ChunkData, Composition, GameData, GameState, PlayerPosition, UnloadChunk,
    WorldChunk, WorldCoord, generate_chunk,
};

#[derive(Resource)]
pub struct ChunkManager {
    pub loaded_chunks: HashMap<ChunkCoord, WorldChunk>,
    pub load_radius: u32,
    pub player_chunk: ChunkCoord,
    pub seed: u32,
    sent_chunks: HashSet<ChunkCoord>,
}

impl ChunkManager {
    pub fn new(load_radius: u32, seed: u32) -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            load_radius,
            player_chunk: ChunkCoord { x: 0, y: 0 },
            seed,
            sent_chunks: HashSet::new(),
        }
    }

    pub fn get_tile(&self, wc: WorldCoord) -> Option<&Composition> {
        let cc = wc.chunk();
        let chunk = self.loaded_chunks.get(&cc)?;
        let (lx, ly) = cc.local(wc);
        if lx < eni_domain::CHUNK_SIZE_U32 && ly < eni_domain::CHUNK_SIZE_U32 {
            Some(chunk.tiles.get(lx, ly))
        } else {
            None
        }
    }
}

/// Stream chunks beyond the currently-loaded single-chunk-manager radius.
/// Not implemented in this pass: real multi-chunk streaming needs
/// persistence and load-balancing that this foundation does not attempt yet.
#[allow(dead_code)]
fn stream_remote_chunks(_chunk_mgr: &mut ChunkManager) {
    todo!("multi-chunk streaming beyond the loaded radius is not implemented")
}

pub fn manage_chunks(
    mut chunk_mgr: ResMut<ChunkManager>,
    player_pos: Res<PlayerPosition>,
    game_data: Res<GameData>,
    mut chunk_data_writer: MessageWriter<ChunkData>,
    mut unload_writer: MessageWriter<UnloadChunk>,
    game_state: Res<State<GameState>>,
) {
    if *game_state.get() != GameState::Playing {
        return;
    }

    let pc = WorldCoord {
        x: player_pos.x.floor() as i32,
        y: player_pos.y.floor() as i32,
    };
    let current_chunk = pc.chunk();

    if current_chunk == chunk_mgr.player_chunk && !chunk_mgr.loaded_chunks.is_empty() {
        return;
    }

    let is_initial = chunk_mgr.loaded_chunks.is_empty();
    let old = if is_initial {
        HashSet::new()
    } else {
        chunks_in_radius(chunk_mgr.player_chunk, chunk_mgr.load_radius)
    };
    let new = chunks_in_radius(current_chunk, chunk_mgr.load_radius);

    for c in old.difference(&new) {
        chunk_mgr.loaded_chunks.remove(c);
        chunk_mgr.sent_chunks.remove(c);
        unload_writer.write(UnloadChunk { chunk_coord: *c });
    }

    let seed = chunk_mgr.seed;
    let mut sent = 0u32;
    for c in new.difference(&old) {
        if !chunk_mgr.loaded_chunks.contains_key(c) {
            let chunk = generate_chunk(seed, *c, &game_data.substances);
            if !chunk_mgr.sent_chunks.contains(c) {
                chunk_data_writer.write(ChunkData::from_chunk(&chunk));
                chunk_mgr.sent_chunks.insert(*c);
                sent += 1;
            }
            chunk_mgr.loaded_chunks.insert(*c, chunk);
        }
    }

    if is_initial {
        tracing::info!(
            chunks = sent,
            radius = chunk_mgr.load_radius,
            "manage_chunks: initial load"
        );
    } else if sent > 0 {
        tracing::info!(chunks = sent, "manage_chunks: sent new chunks");
    }

    chunk_mgr.player_chunk = current_chunk;
}

fn chunks_in_radius(center: ChunkCoord, radius: u32) -> HashSet<ChunkCoord> {
    let r = radius as i32;
    let mut set = HashSet::new();
    for dy in -r..=r {
        for dx in -r..=r {
            set.insert(ChunkCoord {
                x: center.x + dx,
                y: center.y + dy,
            });
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_coord_origin() {
        let c = ChunkCoord { x: 0, y: 0 };
        assert_eq!(c.origin(), WorldCoord { x: 0, y: 0 });

        let c = ChunkCoord { x: 1, y: -1 };
        assert_eq!(c.origin(), WorldCoord { x: 32, y: -32 });

        let c = ChunkCoord { x: -3, y: 7 };
        assert_eq!(c.origin(), WorldCoord { x: -96, y: 224 });
    }

    #[test]
    fn chunks_in_radius_count() {
        assert_eq!(chunks_in_radius(ChunkCoord { x: 0, y: 0 }, 0).len(), 1);
        assert_eq!(chunks_in_radius(ChunkCoord { x: 0, y: 0 }, 1).len(), 9);
        assert_eq!(chunks_in_radius(ChunkCoord { x: 0, y: 0 }, 2).len(), 25);
    }

    #[test]
    fn chunks_in_radius_contains_center() {
        let center = ChunkCoord { x: 5, y: -3 };
        let set = chunks_in_radius(center, 3);
        assert!(set.contains(&center));
    }

    #[test]
    fn chunk_manager_new_defaults() {
        let mgr = ChunkManager::new(5, 42);
        assert!(mgr.loaded_chunks.is_empty());
        assert_eq!(mgr.load_radius, 5);
        assert_eq!(mgr.seed, 42);
        assert_eq!(mgr.player_chunk, ChunkCoord { x: 0, y: 0 });
    }

    #[test]
    fn chunk_manager_get_tile_empty() {
        let mgr = ChunkManager::new(3, 1);
        assert!(mgr.get_tile(WorldCoord { x: 0, y: 0 }).is_none());
    }
}
