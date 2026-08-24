//! Deterministic per-tile terrain/element seeding.
//!
//! Deliberately simple compared to the wuxia project this was forked from:
//! no nations, settlements, or points of interest. Just fractal noise
//! deciding rock / dirt / air / water pockets.

use noise::{Fbm, NoiseFn, Perlin};

use crate::chemistry::{Composition, SubstanceId, SubstanceRegistry};
use crate::chunk::{CHUNK_SIZE_U32, ChunkCoord, TileGrid, VelocityField, WorldChunk};

/// A deterministic 64-bit hash of `(seed, x, y)`, used anywhere generation
/// needs a "random" decision that must reproduce identically for the same
/// input (e.g. deciding which water tiles get a pinch of salt).
pub fn stable_hash(seed: u32, x: i32, y: i32) -> u64 {
    let mut value = u64::from(seed);
    value ^= (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= (y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

const TILE_VOLUME_M3: f32 = 1.0;

fn density_of(registry: &SubstanceRegistry, id: &SubstanceId, fallback: f32) -> f32 {
    registry
        .get(id)
        .map(|def| def.density_kg_per_m3)
        .unwrap_or(fallback)
}

/// Generate one chunk's tile contents from FBM noise, deterministic for a
/// given `(seed, chunk)` pair.
pub fn generate_chunk(seed: u32, chunk: ChunkCoord, registry: &SubstanceRegistry) -> WorldChunk {
    let terrain_fbm = Fbm::<Perlin>::new(seed);
    let water_fbm = Fbm::<Perlin>::new(seed.wrapping_add(1));

    let air = SubstanceId::new("air");
    let water = SubstanceId::new("water");
    let nacl = SubstanceId::new("nacl");
    let rock = SubstanceId::new("rock");
    let dirt = SubstanceId::new("dirt");

    let air_density = density_of(registry, &air, 1.225);
    let water_density = density_of(registry, &water, 1000.0);
    let rock_density = density_of(registry, &rock, 2700.0);
    let dirt_density = density_of(registry, &dirt, 1300.0);

    let mut tiles = TileGrid::new();
    let origin = chunk.origin();

    for ly in 0..CHUNK_SIZE_U32 {
        for lx in 0..CHUNK_SIZE_U32 {
            let wx = origin.x + lx as i32;
            let wy = origin.y + ly as i32;

            let elevation = terrain_fbm.get([wx as f64 * 0.05, wy as f64 * 0.05]);
            let moisture = water_fbm.get([wx as f64 * 0.07, wy as f64 * 0.07]);

            let mut tile = Composition::default();
            if elevation > 0.35 {
                tile.mass_kg
                    .insert(rock.clone(), rock_density * TILE_VOLUME_M3);
            } else if elevation > 0.05 {
                tile.mass_kg
                    .insert(dirt.clone(), dirt_density * TILE_VOLUME_M3);
            } else if moisture > 0.45 {
                let mass = water_density * TILE_VOLUME_M3;
                let salty = stable_hash(seed, wx, wy).is_multiple_of(5);
                if salty {
                    // A pinch of salt: the water/salt mix cannot be trivially
                    // separated back out later (see Composition::mix docs).
                    tile.mass_kg.insert(water.clone(), mass * 0.98);
                    tile.mass_kg.insert(nacl.clone(), mass * 0.02);
                } else {
                    tile.mass_kg.insert(water.clone(), mass);
                }
            } else {
                tile.mass_kg
                    .insert(air.clone(), air_density * TILE_VOLUME_M3);
            }

            tiles.set(lx, ly, tile);
        }
    }

    WorldChunk {
        coord: chunk,
        tiles,
        velocity: VelocityField::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> SubstanceRegistry {
        crate::data::GameData::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data"),
        )
        .expect("bundled data must load")
        .substances
    }

    #[test]
    fn generation_is_deterministic_for_same_seed_and_coord() {
        let registry = test_registry();
        let coord = ChunkCoord { x: 3, y: -5 };
        let a = generate_chunk(42, coord, &registry);
        let b = generate_chunk(42, coord, &registry);
        for (lx, ly, tile_a) in a.tiles.iter() {
            let tile_b = b.tiles.get(lx, ly);
            assert_eq!(tile_a, tile_b);
        }
    }

    #[test]
    fn different_seeds_can_produce_different_chunks() {
        let registry = test_registry();
        let coord = ChunkCoord { x: 0, y: 0 };
        let a = generate_chunk(1, coord, &registry);
        let b = generate_chunk(2, coord, &registry);
        let differs = a
            .tiles
            .iter()
            .zip(b.tiles.iter())
            .any(|((_, _, ta), (_, _, tb))| ta != tb);
        assert!(differs, "different seeds should not always agree");
    }
}
