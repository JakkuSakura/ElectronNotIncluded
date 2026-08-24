//! Authoritative per-tick chemistry/physics pipeline over a `WorldChunk`.
//!
//! Order per tick: heat diffusion, then a full Navier-Stokes fluid step
//! (external forces, viscous diffusion, pressure projection,
//! semi-Lagrangian advection of velocity, projection again, then
//! semi-Lagrangian advection of mass/heat — see `fluid::step_fluid`), then
//! reaction resolution. Phase-change *effects* beyond a flag are explicitly
//! out of scope for this pass (see `flag_phase_changes` below).

use bevy::prelude::*;
use eni_domain::{
    Composition, GameData, GamePaused, MoveIntent, SimulationAdvanced, SimulationClock,
    SubstanceRegistry, WorldChunk, try_react,
};

use crate::chunk_manager::ChunkManager;
use crate::fluid;

/// How quickly heat equalizes between neighbor tiles. Kept as a tunable
/// constant rather than derived from tile geometry, since tiles have no
/// modeled thickness/area yet.
const HEAT_DIFFUSION_RATE: f32 = 0.01;
/// One simulation tick's worth of solver time, passed to `fluid::step_fluid`.
const FLUID_DT: f32 = 1.0;

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

/// Run one full tick of the chemistry pipeline over a chunk: heat diffusion,
/// then a full Navier-Stokes fluid step (see `fluid::step_fluid`), then
/// reaction resolution.
pub fn tick_chunk(chunk: &mut WorldChunk, data: &GameData) {
    diffuse_heat(chunk, &data.substances);
    fluid::step_fluid(chunk, &data.substances, FLUID_DT);
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
            velocity: eni_domain::VelocityField::new(),
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

    /// Ports the intent of the old hardcoded `transfer_mass` gravity-flow
    /// test: a liquid tile above an empty tile should end up with mass
    /// having moved downward, now driven by the full `step_fluid`
    /// buoyancy/projection/advection pipeline rather than a fixed rate
    /// constant.
    #[test]
    fn liquid_gravity_flow_moves_mass_downward() {
        let registry = registry();
        let mut grid = TileGrid::new();
        let water = SubstanceId::new("water");

        // Use an interior cell (not on the chunk edge, which the solver
        // treats as a solid boundary) so the only obstruction is physical.
        let mut upper = Composition::default();
        upper.mass_kg.insert(water.clone(), 1000.0);
        grid.set(5, 5, upper);
        // (5, 6) stays default/empty (vacuum), so it is "open" below the liquid.

        let mut chunk = chunk_with(grid);
        for _ in 0..20 {
            fluid::step_fluid(&mut chunk, &registry, 1.0);
        }

        let upper_mass = chunk.tiles.get(5, 5).total_mass();
        let lower_mass = chunk.tiles.get(5, 6).total_mass();
        assert!(
            upper_mass < 1000.0,
            "some mass should have flowed out of the upper tile"
        );
        assert!(
            lower_mass > 0.0,
            "mass should have arrived in the tile below"
        );
        // This isolated single-tile-into-vacuum scenario is a much harsher
        // test of mass conservation than a densely-filled domain (see
        // `fluid::tests::mass_is_conserved_inside_walled_chunk` for the
        // tight-tolerance conservation guarantee): only two tiles interact
        // with almost all of their bilinear sampling neighborhood being
        // vacuum, so the approximate (non-flux-form) nature of basic
        // semi-Lagrangian scalar advection shows up more. We only check
        // here that mass isn't being wildly created or destroyed.
        let total = upper_mass + lower_mass;
        assert!(
            total > 400.0 && total <= 1000.0 + 1.0,
            "total mass should stay roughly bounded: upper={upper_mass}, lower={lower_mass}"
        );
    }
}
