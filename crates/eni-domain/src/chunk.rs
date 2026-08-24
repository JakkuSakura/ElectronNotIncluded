//! Chunk coordinate system, tile grid, and streaming message types.

use bevy::prelude::*;

use crate::chemistry::Composition;

pub const CHUNK_SIZE: i32 = 32;
pub const CHUNK_SIZE_U32: u32 = 32;
pub const CHUNK_AREA: usize = (CHUNK_SIZE_U32 * CHUNK_SIZE_U32) as usize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub fn try_from_world(wc: WorldCoord) -> Self {
        Self {
            x: wc.x.div_euclid(CHUNK_SIZE),
            y: wc.y.div_euclid(CHUNK_SIZE),
        }
    }

    pub fn origin(self) -> WorldCoord {
        WorldCoord {
            x: self.x * CHUNK_SIZE,
            y: self.y * CHUNK_SIZE,
        }
    }

    pub fn local(self, wc: WorldCoord) -> (u32, u32) {
        let lx = (wc.x - self.x * CHUNK_SIZE) as u32;
        let ly = (wc.y - self.y * CHUNK_SIZE) as u32;
        (lx, ly)
    }

    pub fn contains(self, wc: WorldCoord) -> bool {
        let (lx, ly) = self.local(wc);
        lx < CHUNK_SIZE_U32 && ly < CHUNK_SIZE_U32
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WorldCoord {
    pub x: i32,
    pub y: i32,
}

impl WorldCoord {
    pub fn chunk(self) -> ChunkCoord {
        ChunkCoord::try_from_world(self)
    }
}

/// A `CHUNK_SIZE` x `CHUNK_SIZE` grid of tile chemistry state.
#[derive(Clone, Debug)]
pub struct TileGrid {
    pub data: Vec<Composition>,
}

impl TileGrid {
    pub fn new() -> Self {
        Self {
            data: (0..CHUNK_AREA).map(|_| Composition::default()).collect(),
        }
    }

    fn index(local_x: u32, local_y: u32) -> usize {
        local_y as usize * CHUNK_SIZE_U32 as usize + local_x as usize
    }

    pub fn get(&self, local_x: u32, local_y: u32) -> &Composition {
        &self.data[Self::index(local_x, local_y)]
    }

    pub fn get_mut(&mut self, local_x: u32, local_y: u32) -> &mut Composition {
        &mut self.data[Self::index(local_x, local_y)]
    }

    pub fn set(&mut self, local_x: u32, local_y: u32, tile: Composition) {
        self.data[Self::index(local_x, local_y)] = tile;
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, &Composition)> {
        self.data.iter().enumerate().map(|(i, tile)| {
            let lx = (i % CHUNK_SIZE_U32 as usize) as u32;
            let ly = (i / CHUNK_SIZE_U32 as usize) as u32;
            (lx, ly, tile)
        })
    }
}

impl Default for TileGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct WorldChunk {
    pub coord: ChunkCoord,
    pub tiles: TileGrid,
}

#[derive(Message, Clone, Debug)]
pub struct ChunkData {
    pub chunk_coord: ChunkCoord,
    pub tiles: Vec<Composition>,
}

impl ChunkData {
    pub fn from_chunk(chunk: &WorldChunk) -> Self {
        Self {
            chunk_coord: chunk.coord,
            tiles: chunk.tiles.data.clone(),
        }
    }
}

#[derive(Message, Clone, Copy, Debug)]
pub struct UnloadChunk {
    pub chunk_coord: ChunkCoord,
}

#[derive(Message, Resource, Clone, Copy, Debug)]
pub struct PlayerPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct PlayerMoveIntent {
    pub direction: Vec2,
    pub delta_seconds: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_origin_roundtrip() {
        let wc = WorldCoord { x: -100, y: 50 };
        let cc = ChunkCoord::try_from_world(wc);
        let origin = cc.origin();
        assert!(cc.contains(wc));
        let (lx, ly) = cc.local(wc);
        assert_eq!(origin.x + lx as i32, wc.x);
        assert_eq!(origin.y + ly as i32, wc.y);
    }

    #[test]
    fn negative_coordinates_index_correctly() {
        let wc = WorldCoord { x: -1, y: -1 };
        let cc = ChunkCoord::try_from_world(wc);
        assert_eq!(cc.x, -1);
        assert_eq!(cc.y, -1);
        let (lx, ly) = cc.local(wc);
        assert_eq!(lx, 31);
        assert_eq!(ly, 31);
    }

    #[test]
    fn chunk_exactly_aligned_origin() {
        let wc = WorldCoord { x: -32, y: -32 };
        let cc = ChunkCoord::try_from_world(wc);
        assert_eq!(cc.x, -1);
        assert_eq!(cc.y, -1);
        let (lx, ly) = cc.local(wc);
        assert_eq!(lx, 0);
        assert_eq!(ly, 0);
    }

    #[test]
    fn zero_origin() {
        let wc = WorldCoord { x: 0, y: 0 };
        let cc = ChunkCoord::try_from_world(wc);
        assert_eq!(cc.x, 0);
        assert_eq!(cc.y, 0);
        let (lx, ly) = cc.local(wc);
        assert_eq!(lx, 0);
        assert_eq!(ly, 0);
    }

    #[test]
    fn positive_coordinates() {
        let wc = WorldCoord { x: 65, y: 32 };
        let cc = ChunkCoord::try_from_world(wc);
        assert_eq!(cc.x, 2);
        assert_eq!(cc.y, 1);
        let (lx, ly) = cc.local(wc);
        assert_eq!(lx, 1);
        assert_eq!(ly, 0);
    }

    #[test]
    fn tile_grid_produces_empty_defaults() {
        let grid = TileGrid::new();
        assert_eq!(grid.data.len(), CHUNK_AREA);
        for (_lx, _ly, tile) in grid.iter() {
            assert_eq!(tile.total_mass(), 0.0);
        }
    }

    #[test]
    fn chunk_data_from_chunk_preserves_coord_and_tiles() {
        let mut grid = TileGrid::new();
        let mut tile = Composition::default();
        tile.mass_kg
            .insert(crate::chemistry::SubstanceId::new("rock"), 1234.0);
        grid.set(0, 0, tile);
        let chunk = WorldChunk {
            coord: ChunkCoord { x: 1, y: -2 },
            tiles: grid,
        };
        let data = ChunkData::from_chunk(&chunk);
        assert_eq!(data.chunk_coord, ChunkCoord { x: 1, y: -2 });
        assert_eq!(data.tiles.len(), CHUNK_AREA);
        assert!((data.tiles[0].total_mass() - 1234.0).abs() < f32::EPSILON);
        assert_eq!(data.tiles[CHUNK_AREA - 1].total_mass(), 0.0);
    }
}
