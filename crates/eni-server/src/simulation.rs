//! Authoritative per-tick chemistry/physics pipeline over a `WorldChunk`.
//!
//! Order per tick: heat diffusion, mass transfer (pressure/gravity), then
//! reaction resolution. Phase-change *effects* beyond a flag are explicitly
//! out of scope for this pass (see `flag_phase_changes` below).

use std::collections::HashMap;

use bevy::prelude::*;
use eni_domain::{
    Composition, GameData, GamePaused, MoveIntent, Phase, SimulationAdvanced, SimulationClock,
    SubstanceRegistry, WorldChunk, try_react,
};

use crate::chunk_manager::ChunkManager;

/// How quickly heat equalizes between neighbor tiles. Kept as a tunable
/// constant rather than derived from tile geometry, since tiles have no
/// modeled thickness/area yet.
const HEAT_DIFFUSION_RATE: f32 = 0.01;
/// Fraction of the mass-mismatch that a gas tile equalizes with a
/// lower-mass gas neighbor in one tick.
const GAS_EQUALIZATION_RATE: f32 = 0.1;
/// Fraction of a liquid tile's mass that flows downward per tick when the
/// tile below is not equally-or-more full of liquid.
const LIQUID_GRAVITY_FLOW_RATE: f32 = 0.2;

fn dominant_phase(tile: &Composition, registry: &SubstanceRegistry) -> Option<Phase> {
    tile.mass_kg
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .and_then(|(id, _)| registry.get(id))
        .map(|def| def.phase_at_stp)
}

fn conductivity_of(tile: &Composition, registry: &SubstanceRegistry) -> f32 {
    let total = tile.total_mass();
    if total <= 0.0 {
        return 0.0;
    }
    tile.mass_kg
        .iter()
        .map(|(id, mass)| {
            let conductivity = registry
                .get(id)
                .map(|def| def.thermal_conductivity)
                .unwrap_or(0.5);
            conductivity * mass / total
        })
        .sum()
}

fn orthogonal_neighbors(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::with_capacity(4);
    if x > 0 {
        out.push((x - 1, y));
    }
    if x + 1 < width {
        out.push((x + 1, y));
    }
    if y > 0 {
        out.push((x, y - 1));
    }
    if y + 1 < height {
        out.push((x, y + 1));
    }
    out
}

/// Diffuse heat between orthogonal neighbors proportional to their combined
/// thermal conductivity and the temperature delta between them.
pub fn diffuse_heat(chunk: &mut WorldChunk, registry: &SubstanceRegistry) {
    let width = eni_domain::CHUNK_SIZE_U32;
    let height = eni_domain::CHUNK_SIZE_U32;
    let snapshot = chunk.tiles.clone();

    for ly in 0..height {
        for lx in 0..width {
            let tile = snapshot.get(lx, ly);
            let conductivity = conductivity_of(tile, registry);
            let mut delta_t = 0.0f32;
            for (nx, ny) in orthogonal_neighbors(lx, ly, width, height) {
                let neighbor = snapshot.get(nx, ny);
                let avg_conductivity = (conductivity + conductivity_of(neighbor, registry)) * 0.5;
                delta_t += avg_conductivity
                    * (neighbor.temperature_k - tile.temperature_k)
                    * HEAT_DIFFUSION_RATE;
            }
            chunk.tiles.get_mut(lx, ly).temperature_k += delta_t;
        }
    }
}

/// Move a requested mass (limited to the source's available mass) from one
/// tile to another, mixing it into the destination rather than overwriting
/// it (mixing is never reversible, see `Composition::mix`).
fn move_mass(
    chunk: &mut WorldChunk,
    from: (u32, u32),
    to: (u32, u32),
    amount_kg: f32,
    registry: &SubstanceRegistry,
) {
    if amount_kg <= 0.0 {
        return;
    }
    let from_total = chunk.tiles.get(from.0, from.1).total_mass();
    if from_total <= 0.0 {
        return;
    }
    let fraction = (amount_kg / from_total).min(1.0);
    let temperature_k = chunk.tiles.get(from.0, from.1).temperature_k;

    let mut moved = Composition {
        mass_kg: HashMap::new(),
        temperature_k,
    };
    {
        let source = chunk.tiles.get_mut(from.0, from.1);
        for (id, mass) in source.mass_kg.iter_mut() {
            let delta = *mass * fraction;
            moved.mass_kg.insert(id.clone(), delta);
            *mass -= delta;
        }
        source.mass_kg.retain(|_, m| *m > 1e-6);
    }
    chunk.tiles.get_mut(to.0, to.1).mix(moved, registry);
}

/// Pressure-driven mass transfer, simplified to two rules:
/// - Gas tiles equalize mass with lower-mass orthogonal gas neighbors each
///   tick. This is a stand-in for solving a real pressure field, which would
///   need velocity state per tile; it still converges toward uniform density.
/// - Liquid tiles gravity-flow a fraction of their mass downward into a
///   neighbor below that has less mass or is gas/vacuum.
pub fn transfer_mass(chunk: &mut WorldChunk, registry: &SubstanceRegistry) {
    let width = eni_domain::CHUNK_SIZE_U32;
    let height = eni_domain::CHUNK_SIZE_U32;
    let snapshot = chunk.tiles.clone();

    for ly in 0..height {
        for lx in 0..width {
            let tile = snapshot.get(lx, ly);
            match dominant_phase(tile, registry) {
                Some(Phase::Gas) => {
                    for (nx, ny) in orthogonal_neighbors(lx, ly, width, height) {
                        let neighbor = snapshot.get(nx, ny);
                        if dominant_phase(neighbor, registry) != Some(Phase::Gas) {
                            continue;
                        }
                        if neighbor.total_mass() < tile.total_mass() {
                            let diff = tile.total_mass() - neighbor.total_mass();
                            move_mass(
                                chunk,
                                (lx, ly),
                                (nx, ny),
                                diff * GAS_EQUALIZATION_RATE,
                                registry,
                            );
                        }
                    }
                }
                Some(Phase::Liquid) if ly + 1 < height => {
                    let below = snapshot.get(lx, ly + 1);
                    let below_is_open = dominant_phase(below, registry) != Some(Phase::Liquid)
                        || below.total_mass() < tile.total_mass();
                    if below_is_open {
                        move_mass(
                            chunk,
                            (lx, ly),
                            (lx, ly + 1),
                            tile.total_mass() * LIQUID_GRAVITY_FLOW_RATE,
                            registry,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// Flip a phase flag when a tile's dominant substance crosses its melting or
/// boiling point.
///
/// NOT called from `tick_chunk`: real phase-change effects (latent heat,
/// volume/density change, buoyancy) are out of scope for this pass. This is
/// left as an explicit `todo!()` rather than silently doing nothing, so the
/// gap is visible when this feature becomes needed. It is not reachable from
/// any test or the running simulation.
#[allow(dead_code)]
fn apply_phase_change_effects(_chunk: &mut WorldChunk, _registry: &SubstanceRegistry) {
    todo!("phase-change effects beyond detecting a crossed melting/boiling point are future work")
}

/// Resolve chemical reactions per tile using the loaded reaction rules.
pub fn resolve_reactions(chunk: &mut WorldChunk, reactions: &[eni_domain::ReactionRule]) {
    for tile in chunk.tiles.data.iter_mut() {
        if let Some(result) = try_react(tile, reactions) {
            *tile = result;
        }
    }
}

/// Run one full tick of the chemistry pipeline over a chunk.
pub fn tick_chunk(chunk: &mut WorldChunk, data: &GameData) {
    diffuse_heat(chunk, &data.substances);
    transfer_mass(chunk, &data.substances);
    resolve_reactions(chunk, &data.reactions);
}

pub(crate) fn advance_simulation(
    time: Res<Time<Fixed>>,
    mut clock: ResMut<SimulationClock>,
    paused: Res<GamePaused>,
    mut intent_events: MessageReader<MoveIntent>,
    mut advanced_events: MessageWriter<SimulationAdvanced>,
    mut chunk_mgr: ResMut<ChunkManager>,
    data: Res<GameData>,
) {
    if paused.0 {
        return;
    }
    let _ = time.delta_secs();
    for intent in intent_events.read() {
        if intent.direction != IVec2::ZERO {
            clock.tick = clock.tick.saturating_add(1);
        }
    }
    for chunk in chunk_mgr.loaded_chunks.values_mut() {
        tick_chunk(chunk, &data);
    }
    advanced_events.write(SimulationAdvanced { tick: clock.tick });
}

#[cfg(test)]
mod tests {
    use super::*;
    use eni_domain::{ChunkCoord, SubstanceId, TileGrid};

    fn registry() -> SubstanceRegistry {
        eni_domain::GameData::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data"),
        )
        .expect("bundled data must load")
        .substances
    }

    fn chunk_with(tiles: TileGrid) -> WorldChunk {
        WorldChunk {
            coord: ChunkCoord { x: 0, y: 0 },
            tiles,
        }
    }

    #[test]
    fn heat_diffusion_moves_adjacent_temperatures_closer() {
        let registry = registry();
        let mut grid = TileGrid::new();
        let rock = SubstanceId::new("rock");

        let mut hot = Composition::default();
        hot.mass_kg.insert(rock.clone(), 2700.0);
        hot.temperature_k = 500.0;
        grid.set(0, 0, hot);

        let mut cold = Composition::default();
        cold.mass_kg.insert(rock.clone(), 2700.0);
        cold.temperature_k = 300.0;
        grid.set(1, 0, cold);

        let mut chunk = chunk_with(grid);
        diffuse_heat(&mut chunk, &registry);

        let new_hot = chunk.tiles.get(0, 0).temperature_k;
        let new_cold = chunk.tiles.get(1, 0).temperature_k;
        assert!(
            new_hot < 500.0,
            "hot tile should cool toward the cold neighbor"
        );
        assert!(
            new_cold > 300.0,
            "cold tile should warm toward the hot neighbor"
        );
        assert!(
            (new_hot - new_cold).abs() < 200.0,
            "tiles should move closer together"
        );
    }

    #[test]
    fn liquid_gravity_flow_moves_mass_downward() {
        let registry = registry();
        let mut grid = TileGrid::new();
        let water = SubstanceId::new("water");

        let mut upper = Composition::default();
        upper.mass_kg.insert(water.clone(), 1000.0);
        grid.set(0, 0, upper);
        // (0, 1) stays default/empty (vacuum), so it is "open" below the liquid.

        let mut chunk = chunk_with(grid);
        transfer_mass(&mut chunk, &registry);

        let upper_mass = chunk.tiles.get(0, 0).total_mass();
        let lower_mass = chunk.tiles.get(0, 1).total_mass();
        assert!(
            upper_mass < 1000.0,
            "some mass should have flowed out of the upper tile"
        );
        assert!(
            lower_mass > 0.0,
            "mass should have arrived in the tile below"
        );
        assert!(
            (upper_mass + lower_mass - 1000.0).abs() < 1e-3,
            "total mass must be conserved"
        );
    }
}
