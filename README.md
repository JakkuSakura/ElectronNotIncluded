# ElectronNotIncluded

`eni` is a 2D tile-based sandbox in the spirit of Oxygen Not Included, built
around an advanced, realistic chemistry and fluid-dynamics simulation.
Elements react when they touch and mix into new mixtures rather than being
trivially separable back into their pure components (water + NaCl becomes
diluted saltwater, not "water" and "salt" you can toggle apart). Devices and
machines are out of scope for this foundational pass.

## Development environment

- Rust nightly (see `rust-toolchain.toml`)
- Bevy 0.19
- Optional: Just

## Quick start

```bash
cargo run -p eni
```

Common commands:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
```

CLI runtime tools:

```bash
cargo run -p eni -- verify
cargo run -p eni -- headless --seconds 12.5
cargo run -p eni -- render --output target/preview/world_runtime.png
cargo run -p eni -- operate advance-time --seconds 12.5
cargo run -p eni -- serve --bind 127.0.0.1:3000
```

With no subcommand, the desktop client starts. `verify` loads and validates
the substance/reaction JSON and generates a chunk; `headless` runs the
domain/server runtime without a window; `render` writes a PNG of the
generated chunk colored by dominant substance. The Axum REST server exposes:

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/health` | Liveness check |
| `GET` | `/api/state` | Current runtime snapshot (calendar, paused, world summary) |
| `GET` | `/api/world` | World/chunk summary (seed, substance/reaction counts, total mass) |
| `GET` | `/api/render/world.png` | PNG of the chunk, colored by dominant substance |
| `GET` | `/api/render/framebuffer.png` | Real Bevy offscreen framebuffer capture (desktop runtime only) |
| `POST` | `/api/operate` | Body e.g. `{ "operation": "advance_time", "seconds": 12.5 }` |
| `POST` | `/api/control/state` | Body e.g. `{ "state": "playing" }` |

## Workspace structure

| Crate | Purpose |
| --- | --- |
| `eni-domain` | Shared components, resources, messages, chemistry model, chunk/tile grid |
| `eni-server` | In-process authoritative simulation: chunk streaming, movement, chemistry tick |
| `eni-client` | Bevy client presentation: tile rendering and minimal UI |
| `eni` | Desktop launcher, CLI, and REST server |

Detailed conventions live in [`AGENTS.md`](AGENTS.md).

## Chemistry data

Game content loads from JSON:

- `assets/data/substances.json`: substance properties (phase at STP, density,
  specific heat, melting/boiling points, thermal conductivity).
- `assets/data/reactions.json`: reaction rules (reactants, optional
  temperature window, products, energy delta).

`eni_domain::GameData::load` reads and validates both files. Add new
chemistry by editing these files first.

## Simulation

Each server tick runs, per loaded chunk:

1. **Heat diffusion** between orthogonal tile neighbors, proportional to
   combined thermal conductivity and temperature delta.
2. **Fluid step** (`eni_server::fluid::step_fluid`): a full incompressible
   Navier-Stokes solver using Jos Stam's "Stable Fluids" method — external
   forces (buoyancy/gravity), viscous diffusion, pressure projection,
   semi-Lagrangian advection of velocity, projection again, then
   semi-Lagrangian advection of per-tile mass/temperature through the
   resulting divergence-free velocity field. One mixture velocity field per
   tile; solid tiles and the chunk edge are treated as zero-flow boundaries.
3. **Reaction resolution**: each tile's contents are checked against the
   loaded reaction rules.

Mixing (`Composition::mix`) always merges mass and blends temperature by heat
capacity — there is no "un-mix" operation.

## What's deferred

This is a foundational pass. Explicitly out of scope for now:

- Devices and machines.
- Cross-chunk fluid flow, vorticity confinement, and exact reflective
  boundary conditions at solid interfaces (the fluid solver runs per single
  chunk with the chunk edge treated as a wall; see `eni-server/src/fluid.rs`).
- Multi-chunk streaming/persistence beyond a single loaded chunk radius.
- Phase-change *effects* beyond detecting a crossed melting/boiling point.
- Client rendering polish (tiles are flat colored squares).

## Fonts

The project bundles `ZCOOL XiaoWei` under `assets/fonts` (SIL Open Font
License 1.1, see `OFL.txt`), inherited from the project this was forked from.
It is not currently used by any in-game text.
